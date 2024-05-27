use std::fs;
use serde::Deserialize;
use toml;

use crate::structs::{Material, Sphere};
use crate::structs::Background;

#[derive(Debug, Deserialize)]
pub struct Textureset {
    pub diffuse_path: Option<String>,
    pub normal_path: Option<String>,
    pub roughness_path: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ModelPaths {
    pub gltf_path: Option<String>,
    pub obj_path: Option<String>,
    pub obj_material_id: Option<i32>,
}

impl ModelPaths {
    pub fn new(gltf_path: Option<String>, obj_path: Option<String>, obj_material_id: Option<i32>) -> Self {
        Self {
            gltf_path,
            obj_path,
            obj_material_id,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub camera_position: [f32; 3],
    pub camera_rotation: [f32; 2],
    pub camera_near_far: [f32; 2],
    pub camera_fov: f32,

    pub materials: Option<Vec<Material>>,
    pub textures: Option<Vec<Textureset>>,
    pub background: Option<Background>,
    pub background_path: Option<String>,

    pub spheres: Option<Vec<Sphere>>,
    #[serde(rename = "3d_model_paths")]
    pub model_paths: ModelPaths,
}

impl Config {
    pub fn new(config_path: &str) -> Result<Self, String> {
        let toml_str = fs::read_to_string(config_path)
            .map_err(|e| format!("Could not find/read config file: {}", e))?;
        Self::from_str(&toml_str)
    }

    pub fn from_str(toml_str: &str) -> Result<Self, String> {
        let toml: toml::Value = toml::from_str(toml_str)
            .map_err(|e| format!("Could not parse TOML: {}", e))?;

        // Extract required fields for Config struct
        let toml_camera = toml.get("camera").ok_or("Missing camera section")?;
        let camera_position_vec = parse_array(toml_camera.get("position").ok_or("Missing camera position")?)?;
        let camera_position = [camera_position_vec[0], camera_position_vec[1], camera_position_vec[2]];
        let camera_rotation_vec = parse_array(toml_camera.get("rotation").ok_or("Missing camera rotation")?)?;
        let camera_rotation = [camera_rotation_vec[0], camera_rotation_vec[1]];
        // Near and far aren't critical and only really needed in edge cases, so we can use defaults if they're missing making the values optional
        let toml_camera_near_far_vec = toml_camera.get("near_far");
        let camera_near_far_vec = match toml_camera_near_far_vec {
            Some(value) => parse_array(value)?,
            None => {
                println!("No near_far defined in config, using default values");
                vec![0.1, 100.0]
            },
        };
            
        let camera_near_far = [camera_near_far_vec[0], camera_near_far_vec[1]];
        let camera_fov = toml_camera.get("fov").ok_or("Missing camera fov")?.as_float().ok_or("Expected float for camera fov")? as f32;

        // Materials
        let materials = load_materials_config(toml.get("materials"))?;

        // Textures
        let textures = match load_textures_config(toml.get("textures")) {
            Ok(textures) => textures,
            Err(e) => {
                println!("Failed to load textures: {}", e);
                None
            }
        };
        let (background, background_path) = load_background_config(toml.get("background"))?;

        // Spheres
        let spheres = load_spheres_config(toml.get("spheres"))?;

        // 3D Models
        let model_paths = load_3d_models_config(toml.get("3d_model_paths"))?;

        Ok(Self {
            camera_position,
            camera_rotation,
            camera_near_far,
            camera_fov,

            materials,
            textures,
            background,
            background_path,

            spheres,
            model_paths,
        })
    }
}

fn parse_array(value: &toml::Value) -> Result<Vec<f32>, String> {
    let array = value.as_array().ok_or("Expected array")?;
    let result = array.iter()
        .map(|v| v.as_float().ok_or("Expected float").map(|f| f as f32))
        .collect::<Result<Vec<f32>, _>>()?;
    Ok(result)
}

// makes materials optional in config
fn load_materials_config(value: Option<&toml::Value>) -> Result<Option<Vec<Material>>, String> {
    match value {
        Some(value) => {
            let array = value.as_array().ok_or("Expected array for materials")?;
            let materials = array.iter().map(|v| {
                let mut v = v.clone();
                // Make color and attenuation 4 elements instead of 3
                let mut color = v.get("color").ok_or("Missing color")?.as_array().ok_or("Expected array for color")?.clone();
                let mut attenuation = v.get("attenuation").ok_or("Missing attenuation")?.as_array().ok_or("Expected array for attenuation")?.clone();

                // Add a fourth element to color and attenuation
                color.push(toml::Value::Float(0.0));
                attenuation.push(toml::Value::Float(0.0));

                // Update the color and attenuation in v
                v.as_table_mut().unwrap().insert("color".to_string(), toml::Value::Array(color));
                v.as_table_mut().unwrap().insert("attenuation".to_string(), toml::Value::Array(attenuation));
                v.as_table_mut().unwrap().insert("__padding".to_string(), toml::Value::Float(0.0));

                // Convert v to Material
                v.try_into().map_err(|_| "Could not convert to Material")
            }).collect::<Result<Vec<Material>, _>>()?;
            Ok(Some(materials))
        },
        None => {
            println!("No materials defined in config");
            Ok(None)
        }
    }
}
// makes textures optional in config
fn load_textures_config(value: Option<&toml::Value>) -> Result<Option<Vec<Textureset>>, String> {
    match value {
        Some(value) => {  
            let array = value.as_array().ok_or("Expected array for textures")?;
            let textures = array.iter().map(|v| {
                let diffuse = v.get("diffuse").and_then(|v| v.as_str()).map(|v| v.to_string());
                let normal = v.get("normal").and_then(|v| v.as_str()).map(|v| v.to_string());
                let roughness = v.get("roughness").and_then(|v| v.as_str()).map(|v| v.to_string());
                if diffuse.is_some() || normal.is_some() || roughness.is_some() {
                    Ok(Textureset {
                        diffuse_path: diffuse,
                        normal_path: normal,
                        roughness_path: roughness,
                    })
                } else {
                    Err("Missing texture paths".to_string())
                }
            }).collect::<Result<Vec<Textureset>, _>>()?;
            Ok(Some(textures))
        },
        None => {
            println!("No textures defined in config");
            Ok(None)
        }
    }
}

// makes background optional in config
fn load_background_config(value: Option<&toml::Value>) -> Result<(Option<Background>, Option<String>), String> {
    match value {
        Some(value) => {
            // if v is empty, meaning no sphere is defined, return none
            if value.as_table().is_none() || (value.as_table().is_some() && value.as_table().unwrap().is_empty()) {
                return Ok((None,None));
            }
            let material_id = value.get("material_id").and_then(|v| v.as_integer()).map(|v| v as i32);
            let background_path = value.get("background_path").and_then(|v| v.as_str()).map(|v| v.to_string());
            let intensity = value.get("intensity").and_then(|v| v.as_float()).map(|v| v as f32);

            if let (Some(material_id), Some(background_path), Some(intensity)) = (material_id, background_path.clone(), intensity) {
                println!("Background defined in config");
                Ok((
                    Some(Background::new(
                        material_id,
                        0,
                        intensity,
                    )), 
                    Some(background_path)
                ))
            } else if let (Some(material_id), Some(intensity)) = (material_id, intensity) {
                println!("Background defined without path in config");
                Ok((
                    Some(Background::new(
                        material_id,
                        0,
                        intensity,
                    )), 
                    None
                ))
            } else {
                print!("material_id: {:?}, background_path: {:?}, intensity: {:?}", material_id, background_path, intensity);
                Err("Missing or invalid fields in background config".to_string())
            }
        },
        None => {
            println!("No background defined in config");
            Ok((None, None))
        }
    }
}




// makes 3D models optional in config
fn load_3d_models_config(value: Option<&toml::Value>) -> Result<ModelPaths, String> {
    match value {
        Some(value) => {
            let gltf_path = value.get("gltf_path").and_then(|v| v.as_str()).map(|v| v.to_string());
            let obj_path = value.get("obj_path").and_then(|v| v.as_str()).map(|v| v.to_string());
            let obj_material_id = value.get("obj_material_id").and_then(|v| v.as_integer()).map(|v| v as i32);
            Ok(ModelPaths::new(gltf_path, obj_path, obj_material_id))
        },
        None => {
            println!("No 3D model paths defined in config");
            Ok(ModelPaths::default())
        }
    }
}

// makes spheres optional in config
fn load_spheres_config(value: Option<&toml::Value>) -> Result<Option<Vec<Sphere>>, String> {
    match value {
        Some(value) => {
            let value = value.as_array().ok_or("Expected array")?
                .iter()
                .map(|v| {
                    // if v is empty, meaning no sphere is defined, return none
                    if v.as_table().is_none() || (v.as_table().is_some() && v.as_table().unwrap().is_empty()) {
                        return Ok(None);
                    }

                    let mut v = v.clone();
                    let mut position = v.get("position").ok_or("Missing position")?.as_array().ok_or("Expected array")?.clone();

                    let texture_id: Vec<f32> = v.get("texture_id").ok_or("Missing texture_id")?.as_array().ok_or("Expected array")?
                        .iter()
                        .map(|value: &toml::Value| value.as_integer().ok_or("Expected int"))
                        .map(|value: Result<i64, &str>| value.map(|value| value as f32))
                        .collect::<Result<Vec<f32>, _>>()?;

                    let radius = v.get("radius").ok_or("Missing radius")?.as_float().ok_or("Expected float")? as f32;
                    let material_id = v.get("material_id").ok_or("Missing material_id")?.as_integer().ok_or("Expected int")? as f32;

                    // Fix length of arrays
                    let radius_array = vec![radius, 0.0, 0.0, 0.0].iter().map(|&value| toml::Value::Float(value as f64)).collect::<Vec<toml::Value>>();

                    position.push(toml::Value::Float(0.0));
                    let material_texture_id = [
                        material_id,
                        texture_id[0],
                        texture_id[1],
                        texture_id[2],
                    ].iter().map(|&value| toml::Value::Float(value as f64)).collect::<Vec<toml::Value>>();

                    // Update the color and attenuation in v
                    v.as_table_mut().unwrap().insert("center".to_string(), toml::Value::Array(position));
                    v.as_table_mut().unwrap().insert("radius".to_string(), toml::Value::Array(radius_array));
                    v.as_table_mut().unwrap().insert("material_texture_id".to_string(), toml::Value::Array(material_texture_id));

                    // Convert v to Material
                    v.try_into().map_err(|_| "Could not convert to Material".to_string())
                }).collect::<Result<Option<Vec<Sphere>>, String>>()?;
            Ok(value)
        },
        None => {
            println!("No spheres defined in config");
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_missing() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0\n[[materials]]
            \ncolor = [1.0, 0.0, 0.0]\nattenuation = [0.1, 0.1, 0.1]\n[[textures]]\ndiffuse = \"path/to/diffuse.png\"\nnormal = \"path/to/normal.png\"\nroughness = \"path/to/roughness.png\"
            \n[background]\nmaterial_id = 1\nbackground_path = \"path/to/background.png\"\nintensity = 0.5\n[[spheres]]\nposition = [0.0, 0.0, 0.0]\nradius = 1.0\ntexture_id = [0, 1, 2]
            \nmaterial_id = 0\n[3d_model_paths]\ngltf_path = \"path/to/model.gltf\"\nobj_path = \"path/to/model.obj\"\nobj_material_id = 1\n");
        assert!(config.is_err());
    }

    #[test]
    fn test_camera_missing_position() {
        let config = Config::from_str("[camera]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0");
        assert!(config.is_err());
    }

    #[test]
    fn test_camera_missing_rotation() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nnear_far = [0.1, 100.0]\nfov = 45.0");
        assert!(config.is_err());
    }

    #[test]
    fn test_camera_missing_near_far() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nfov = 45.0");
        assert!(config.is_ok());
        let config = config.expect("Could not unwrap config");
        assert!(config.camera_near_far == [0.1, 100.0]);
    }

    #[test]
    fn test_camera_missing_fov() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]");
        assert!(config.is_err());
    }

    // Materials tests
    #[test]
    fn test_materials_missing() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0");
        assert!(config.is_ok());
        let config = config.expect("Could not unwrap config");
        assert!(config.materials.is_none());
    }

    #[test]
    fn test_materials_empty() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0\n[[materials]]");
        assert!(config.is_err());
    }

    #[test]
    fn test_materials_one_material() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0\n[[materials]]\ncolor = [1.0, 0.0, 0.0]\nattenuation = [0.1, 0.1, 0.1]\nroughness = 0.2\nemission = 0.0\nior = 0.0");
        assert!(config.is_ok());
        let config = config.expect("Could not unwrap config");
        
        assert!(config.materials.is_some());
        let materials = config.materials.unwrap();
        assert_eq!(materials.len(), 1);
        assert_eq!(materials[0].albedo, [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(materials[0].attenuation, [0.1, 0.1, 0.1, 0.0]);
        assert_eq!(materials[0].roughness, 0.2);
        assert_eq!(materials[0].emission, 0.0);
    }

    #[test]
    fn test_materials_material_array() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0\n[[materials]]\ncolor = [1.0, 0.0, 0.0]\nattenuation = [0.1, 0.1, 0.1]\nroughness = 0.2\nemission = 0.0\nior = 0.0\n[[materials]]\ncolor = [0.0, 1.0, 0.0]\nattenuation = [0.2, 0.2, 0.2]\nroughness = 0.3\nemission = 0.0\nior = 0.0");
        assert!(config.is_ok());
        let config = config.expect("Could not unwrap config");
        
        assert!(config.materials.is_some());
        let materials = config.materials.unwrap();
        assert_eq!(materials.len(), 2);
        assert_eq!(materials[0].albedo, [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(materials[0].attenuation, [0.1, 0.1, 0.1, 0.0]);
        assert_eq!(materials[1].albedo, [0.0, 1.0, 0.0, 0.0]);
        assert_eq!(materials[1].attenuation, [0.2, 0.2, 0.2, 0.0]);
    }

    #[test]
    fn test_materials_missing_fields() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0\n[[materials]]\nattenuation = [0.1, 0.1, 0.1]");
        assert!(config.is_err());
    }

    // Textures tests
    #[test]
    fn test_textures_missing() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0");
        assert!(config.is_ok());
        let config = config.expect("Could not unwrap config");
        assert!(config.textures.is_none());
    }

    #[test]
    fn test_textures_empty() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0\n[[textures]]");
        assert!(config.is_ok());
        let config = config.expect("Could not unwrap config");
        println!("{:?}", config.textures);
        assert!(config.textures.is_none());
    }

    #[test]
    fn test_textures_correct() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0\n[[textures]]\ndiffuse = \"path/to/diffuse.png\"\nnormal = \"path/to/normal.png\"\nroughness = \"path/to/roughness.png\"");
        assert!(config.is_ok());
        let config = config.expect("Could not unwrap config");
        
        assert!(config.textures.is_some());
        let textures = config.textures.unwrap();
        assert_eq!(textures.len(), 1);
        assert_eq!(textures[0].diffuse_path.as_deref(), Some("path/to/diffuse.png"));
        assert_eq!(textures[0].normal_path.as_deref(), Some("path/to/normal.png"));
        assert_eq!(textures[0].roughness_path.as_deref(), Some("path/to/roughness.png"));
    }

    #[test]
    fn test_textures_missing_fields() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0\n[[textures]]\ndiffuse = \"path/to/diffuse.png\"");
        assert!(config.is_ok());
        let config = config.expect("Could not unwrap config");
        assert!(config.textures.is_some());
        let textures = config.textures.unwrap();
        assert_eq!(textures.len(), 1);
    }

    #[test]
    fn test_spheres_correct() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0\n[[spheres]]\nposition = [0.0, 0.0, 0.0]\nradius = 1.0\ntexture_id = [0, 1, 2]\nmaterial_id = 0");
        assert!(config.is_ok());
        let config = config.expect("Could not unwrap config");
        
        assert!(config.spheres.is_some());
        let spheres = config.spheres.unwrap();
        assert_eq!(spheres.len(), 1);
        assert_eq!(spheres[0].center, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(spheres[0].radius, [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(spheres[0].material_texture_id, [0.0, 0.0, 1.0, 2.0]);
    }

    #[test]
    fn test_spheres_empty() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0\n[[spheres]]");
        assert!(config.is_ok());
        let config = config.expect("Could not unwrap config");
        assert!(config.spheres.is_none());
    }

    #[test]
    fn test_spheres_missing_fields() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0\n[[spheres]]\nposition = [0.0, 0.0, 0.0]\nradius = 1.0");
        assert!(config.is_err());
    }

    #[test]
    fn test_spheres_missing() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0");
        assert!(config.is_ok());
        let config = config.expect("Could not unwrap config");
        assert!(config.spheres.is_none());
    }

    #[test]
    fn test_background_correct() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0\n[background]\nmaterial_id = 1\nbackground_path = \"path/to/background.png\"\nintensity = 0.5");
        assert!(config.is_ok());
        let config = config.expect("Could not unwrap config");
        
        assert!(config.background.is_some());
        let background = config.background.unwrap();
        assert_eq!(background.material_texture_id[0], 1.0);
        assert_eq!(config.background_path.as_deref(), Some("path/to/background.png"));
        assert_eq!(config.background.unwrap().intensity, 0.5);
    }

    #[test]
    fn test_background_missing_fields() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0\n[background]\nmaterial_id = 1\nintensity = 0.5");
        assert!(config.is_ok());
        let config = config.expect("Could not unwrap config");
        assert!(config.background_path.is_none());
    }

    #[test]
    fn test_background_missing() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0");
        assert!(config.is_ok());
        let config = config.expect("Could not unwrap config");
        assert!(config.background.is_none());
    }

    #[test]
    fn test_background_empty() {
        let config = Config::from_str("[camera]\nposition = [0.0, 1.0, 2.0]\nrotation = [0.0, 0.0]\nnear_far = [0.1, 100.0]\nfov = 45.0\n[background]");
        assert!(config.is_ok());
        let config = config.expect("Could not unwrap config");
        assert!(config.background.is_none());
    }
}
