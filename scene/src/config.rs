use std::fs;
use serde::Deserialize;
use toml;

use crate::structs::{Material, Sphere};
use crate::structs::Background;
#[derive(Debug, Deserialize)]
pub struct Texture {
    pub paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ModelPaths {
    pub gltf_path: Option<String>,
    pub obj_path: Option<String>,
}

impl ModelPaths {
    pub fn new(gltf_path: Option<String>, obj_path: Option<String>) -> Self {
        Self {
            gltf_path,
            obj_path,
        }
    }

    pub fn default() -> Self {
        Self {
            gltf_path: None,
            obj_path: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub camera_position: [f32; 3],
    pub camera_rotation: [f32; 2],
    pub camera_near_far: [f32; 2],
    pub camera_fov: f32,

    pub materials: Option<Vec<Material>>,
    pub textures: Option<Vec<Texture>>,
    pub background: Option<Background>,
    pub background_path: Option<String>,

    pub spheres: Option<Vec<Sphere>>,
    #[serde(rename = "3d_model_paths")]
    pub model_paths: ModelPaths,
}

impl Config {
    pub fn new() -> Self {
        let toml_str = fs::read_to_string("res/Config.toml")
            .expect("Could not find/read config file");

        let toml: toml::Value = toml::from_str(&toml_str)
        .expect("Could not parse TOML");

        // Extract required fields for Config struct
        let toml_camera = toml.get("camera").expect("Missing camera");
        let camera_position_vec = parse_array(toml_camera.get("position").expect("Missing camera position"));
        let camera_position = [camera_position_vec[0], camera_position_vec[1], camera_position_vec[2]];
        let camera_rotation_vec = parse_array(toml_camera.get("rotation").expect("Missing camera rotation"));
        let camera_rotation = [camera_rotation_vec[0], camera_rotation_vec[1]];
        let camera_near_far_vec = parse_array(toml_camera.get("near_far").expect("Missing camera near_far"));
        let camera_near_far = [camera_near_far_vec[0], camera_near_far_vec[1]];
        let camera_fov = toml_camera.get("fov").expect("Missing camera fov").as_float().expect("Expected float") as f32;

        // Materials
        let materials = load_materials_config(toml.get("materials"));

        // Textures
        let textures = load_textures_config(toml.get("textures"));
        let (background,background_path)  = load_background_config(toml.get("background"));

        // Spheres
        let spheres = load_spheres_config(toml.get("spheres"));

        // 3D Models
        let model_paths = load_3d_models_config(toml.get("3d_model_paths"));

        Self {
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
        }
    }
}

fn parse_array(value: &toml::Value) -> Vec<f32> {
    let array = value.as_array().expect("Expected array");
    let mut result: Vec<f32> = vec![0.0; array.len()];
    for (i, v) in array.iter().enumerate() {
        result[i] = v.as_float().expect("Expected float") as f32;
    }
    return result
}

// makes materials optional in config
fn load_materials_config(value: Option<&toml::Value>) -> Option<Vec<Material>> {
    match value {
        Some(value) => {
            let value = value.as_array().expect("Expected array").iter()
            .map(|v| {
                let mut v = v.clone();
                //make color and attenuation 4 elements instead of 3
                let mut color = v.get("color").expect("Missing color").as_array().expect("Expected array").clone();
                let mut attenuation = v.get("attenuation").expect("Missing attenuation").as_array().expect("Expected array").clone();
                
                // Add a fourth element to color and attenuation
                color.push(toml::Value::Float(0.0));
                attenuation.push(toml::Value::Float(0.0));

                // Update the color and attenuation in v
                v.as_table_mut().unwrap().insert("color".to_string(), toml::Value::Array(color));
                v.as_table_mut().unwrap().insert("attenuation".to_string(), toml::Value::Array(attenuation));
                v.as_table_mut().unwrap().insert("__padding".to_string(), toml::Value::Float(0.0));

                // Convert v to Material
                v.try_into().expect("Could not convert to Material")
            }).collect::<Vec<Material>>();
            return Some(value)
        },
        None => {
            println!("No materials defined in config");
            return None
        }
    }
}

// makes textues optional in config
fn load_textures_config(value: Option<&toml::Value>) -> Option<Vec<Texture>> {
    match value {
        Some(value) => {
            let value = value.as_array().expect("Expected array").iter()
                .map(|v| v.clone().try_into().expect("Could not convert to Texture")).collect();
            Some(value)
        },
        None => {
            println!("No textures defined in config");
            None
        }
    }
}

// makes background optional in config
fn load_background_config(value: Option<&toml::Value>) -> (Option<Background>, Option<String>) {
    match value {
        Some(value) => {
            let material_id = value.get("material_id").and_then(|v| v.as_integer()).map(|v| v as f32);
            let background_path = value.get("background_path").and_then(|v| v.as_str()).map(|v| v.to_string());
            let intensity = value.get("intensity").and_then(|v| v.as_float()).map(|v| v as f32);

            if let (Some(material_id), Some(background_path), Some(intensity)) = (material_id, background_path, intensity) {
                (
                    Some(Background::new(
                        material_id as i32,
                        0,
                        intensity.try_into().unwrap_or_else(|_| 0.0),
                    )), 
                    Some(background_path)
                )
            } else {
                println!("Missing or invalid fields in config");
                (None, None)
            }
        },
        None => {
            println!("No background defined in config");
            (None, None)
        }
    }
}

// makes 3d models optional in config
fn load_3d_models_config(value: Option<&toml::Value>) -> ModelPaths {
    match value {
        Some(value) => {
            let gltf_path = value.get("gltf_path")
                .map_or_else(|| {
                    println!("Missing gltf_path");
                    None
                }, |v| v.as_str().map(|s| s.to_string())).or_else(|| {
                    println!("Can't convert gltf_path to string");
                    None
                });

            let obj_path = value.get("obj_path")
                .map_or_else(|| {
                    println!("Missing obj_path");
                    None
                }, |v| v.as_str().map(|s| s.to_string())).or_else(|| {
                    println!("Can't convert obj_path to string");
                    None
                });

            // gen struct
            ModelPaths::new(
                gltf_path,
                obj_path
            )
        },
        None => {
            println!("No 3d models defined in config");
            ModelPaths::default()
        }
    }
}

// makes spheres optional in config
fn load_spheres_config(value: Option<&toml::Value>) -> Option<Vec<Sphere>> {
    match value {
        Some(value) => {
            let value = value.as_array().expect("Expected array").iter()
                .map(|v| {
                    let mut v = v.clone();
                    let mut position = v.get("position").expect("Missing color").as_array().expect("Expected array").clone();

                    let texture_id: Vec<f32> = v.get("texture_id").expect("Missing texture_id").as_array().expect("Expected array")
                        .iter()
                        .map(|value| value.as_integer().expect("Expected int") as f32)
                        .collect();

                    let radius = v.get("radius").expect("Missing radius").as_float().expect("Expected float") as f32;
                    let material_id = v.get("material_id").expect("Missing material_id").as_integer().expect("Expected int") as f32;

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
                    v.try_into().expect("Could not convert to Material")
                }).collect::<Vec<Sphere>>();
            Some(value)
        },
        None => {
            println!("No spheres defined in config");
            None
        }
    }
}