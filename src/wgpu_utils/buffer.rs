use bytemuck::Pod;
use wgpu::util::DeviceExt;

/// A struct representing the initial descriptor for a buffer.
///
/// This struct is used to create a new buffer with specified label and usage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferInitDescriptor<'a> {
    /// Debug label of a buffer. This will show up in graphics debuggers for easy identification.
    pub label: wgpu::Label<'a>,
    /// Usages of a buffer. If the buffer is used in any way that isn't specified here, the operation
    /// will panic.
    pub usage: wgpu::BufferUsages,
}

impl<'a> BufferInitDescriptor<'a> {
    pub fn new(label: wgpu::Label<'a>, usage: wgpu::BufferUsages) -> Self {
        Self { label, usage }
    }
}

impl<'a> Default for BufferInitDescriptor<'a> {
    fn default() -> Self {
        Self {
            label: Some("Default BufferInitDescriptor"),
            usage: wgpu::BufferUsages::COPY_DST,
        }
    }
}

pub fn create_new_buffer<T: Pod>(device: &wgpu::Device, data: &[T], descriptor: BufferInitDescriptor) -> wgpu::Buffer {
    return device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: descriptor.label,
        contents: bytemuck::cast_slice(data),
        usage: descriptor.usage,
    });    
}



/// An enum representing the template for a binding resource.
/// This shortens the amount of code needed to create a bind group layout and bind group.
///
/// This enum can be one of three variants: `Buffer`, `TextureView`, or `Sampler`.
#[derive(Clone, Debug)]
pub enum BindingResourceTemplate<'a> {
    BufferStorage(wgpu::BindingResource<'a>),
    BufferUniform(wgpu::BindingResource<'a>),
    StorageTexture(wgpu::BindingResource<'a>),
    TextureView(wgpu::BindingResource<'a>),
    Sampler(wgpu::BindingResource<'a>),
}

/// A function to get a `BindingResource` from a `BindingResourceTemplate`.
///
/// This function takes a `BindingResourceTemplate` and returns a `BindingResource`.
pub fn get_binding_resource<'a>(template: BindingResourceTemplate<'a>) -> wgpu::BindingResource<'a> {
    match template {
        BindingResourceTemplate::BufferStorage(binding_resource) => binding_resource,
        BindingResourceTemplate::BufferUniform(binding_resource) => binding_resource,
        BindingResourceTemplate::StorageTexture(binding_resource) => binding_resource,
        BindingResourceTemplate::TextureView(binding_resource) => binding_resource,
        BindingResourceTemplate::Sampler(binding_resource) => binding_resource,
    }
}

/// A struct representing a type of buffer.
/// This enables the user to specify the type of buffer and the view dimension in a compact way.
/// This struct can be piced appart to create a bind group layout and bind group.
///
/// This struct contains a `BindingResourceTemplate` and an optional `TextureViewDimension`.
pub struct BufferType<'a> {
    ty: BindingResourceTemplate<'a>,
    view_dimension: Option<wgpu::TextureViewDimension>,
}

impl PartialEq for BindingResourceTemplate<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (BindingResourceTemplate::BufferStorage(_), BindingResourceTemplate::BufferStorage(_)) => true,
            (BindingResourceTemplate::BufferUniform(_), BindingResourceTemplate::BufferUniform(_)) => true,
            (BindingResourceTemplate::StorageTexture(_), BindingResourceTemplate::StorageTexture(_)) => true,
            (BindingResourceTemplate::TextureView(_), BindingResourceTemplate::TextureView(_)) => true,
            (BindingResourceTemplate::Sampler(_), BindingResourceTemplate::Sampler(_)) => true,
            _ => false,
        }
    }
}

impl<'a> BufferType<'a> {
    pub fn new(ty: BindingResourceTemplate<'a>) -> Self {
        Self { ty, view_dimension: None }
    }

    pub fn with_view_dimension(ty: BindingResourceTemplate<'a>, view_dimension: wgpu::TextureViewDimension) -> Self {
        // Check if the binding type is a texture view or Storage Texture,
        //Other types aren't alowed to have a view dimension
        if let BindingResourceTemplate::TextureView(_) = ty {
            Self { ty, view_dimension: Some(view_dimension) }
        } else if let BindingResourceTemplate::StorageTexture(_) = ty {
            Self { ty, view_dimension: Some(view_dimension) }
        } else{
            panic!("BufferType::with_view_dimension can only be used with BindingResource::TextureView");
        }
    }
}

/// A struct representing a descriptor for a bind group.
/// This struct can be used to create a bind group and bind group layout.
///
/// This struct contains a label, a reference to a `BindGroupLayout`, a `ShaderStages`, and a vector of `BufferType`.
pub struct BindGroupDescriptor<'a> {
    pub label: wgpu::Label<'a>,
    pub layout: Option<wgpu::BindGroupLayout>,
    pub vis: wgpu::ShaderStages,
    pub bindings: Vec<BufferType<'a>>,
}

impl<'a> BindGroupDescriptor<'a> {
    pub fn new (label: wgpu::Label<'a>, vis: wgpu::ShaderStages, bindings: Vec<BufferType<'a>>) -> Self {
        Self { label, layout:None, vis, bindings }
    }

    /// A method to generate a bind group.
    ///
    /// This method takes a reference to a `wgpu::Device` and returns a `wgpu::BindGroup`.
    pub fn generate_bind_group(&mut self, device: &wgpu::Device) -> wgpu::BindGroup {
        //count the number of bindings
        let mut binding_index = 0;
        
        let entries = self.bindings.iter().map(|binding| {
            binding_index += 1;
            wgpu::BindGroupEntry {
                binding: binding_index - 1,
                resource: get_binding_resource(binding.ty.clone())
            }
        }).collect::<Vec<_>>();

        //append _bind_group if lable is Some
        let mod_label = self.label.as_ref().map(|label| format!("{}_bind_group", label));
        //generate bind group layout
        self.generate_bind_group_layout(device);

        //ensure bind group layout is Some
        let bg_layout;
        match &self.layout {
            Some(layout) => bg_layout = layout,
            None => panic!("BindGroupLayout is None"),
        };

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: mod_label.as_deref(),
            layout: bg_layout,
            entries: &entries,
        });

        return bind_group;
    }

    /// A method to generate a bind group layout.
    ///
    /// This method takes a reference to a `wgpu::Device` and returns a `wgpu::BindGroupLayout`.
    pub fn generate_bind_group_layout(&mut self, device: &wgpu::Device) {
        
        //append _bind_group if lable is Some
        let mod_label = self.label.as_ref().map(|label| format!("{}_bind_group_label", label));

        //count the number of bindings
        let mut binding_index = 0;

        self.layout = Some(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: mod_label.as_deref(),
            entries: &self.bindings.iter().map(|binding| {
                binding_index += 1;
                match &binding.ty {
                    BindingResourceTemplate::BufferStorage(_) => {
                        wgpu::BindGroupLayoutEntry {
                            binding: binding_index - 1,
                            visibility: self.vis,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        }
                    }
                    BindingResourceTemplate::BufferUniform(_) => {
                        wgpu::BindGroupLayoutEntry {
                            binding: binding_index - 1,
                            visibility: self.vis,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        }
                    }
                    BindingResourceTemplate::StorageTexture(_) => {
                        wgpu::BindGroupLayoutEntry {
                            binding: binding_index - 1,
                            visibility: self.vis,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::ReadWrite,
                                format: wgpu::TextureFormat::Rgba8Unorm, //update to config.format
                                view_dimension: binding.view_dimension.unwrap(),
                            },
                            count: None,
                        }
                    }
                    BindingResourceTemplate::TextureView(_) => {
                        wgpu::BindGroupLayoutEntry {
                            binding: binding_index - 1,
                            visibility: self.vis,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: binding.view_dimension.unwrap(),
                                multisampled: false,
                            },
                            count: None,
                        }
                    }
                    BindingResourceTemplate::Sampler(_) => {
                        wgpu::BindGroupLayoutEntry {
                            binding: binding_index - 1,
                            visibility: self.vis,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        }
                    } 
                }
            }).collect::<Vec<_>>(),
        }));
    }
}