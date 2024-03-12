use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use rand::Rng;
use crate::structs::{Triangle};

pub fn load_obj(file_path: &str) -> Result<Vec<Triangle>, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut vertices = Vec::new();
    let mut texture_coords = Vec::new();
    let mut normals = Vec::new();
    let mut faces: Vec<Triangle> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.starts_with("v ") {
            // Parse vertex coordinates
            let values: Vec<f32> = line[2..]
                .split_whitespace()
                .map(|x| x.parse::<f32>())
                .collect::<Result<_, _>>()?;

            if values.len() >= 3 {
                let vertex = [values[0], values[1], values[2]];
                vertices.push(vertex);
            }
        } else if line.starts_with("vt ") {
            // Parse texture coordinates
            let values: Vec<f32> = line[3..]
                .split_whitespace()
                .map(|x| x.parse::<f32>())
                .collect::<Result<_, _>>()?;

            if values.len() >= 2 {
                let tex_coord = [values[0], values[1]];
                texture_coords.push(tex_coord);
            }
        } else if line.starts_with("vn ") {
            // Parse normals
            let val: Vec<f32> = line[3..]
                .split_whitespace()
                .map(|x| x.parse::<f32>())
                .collect::<Result<_, _>>()?;

            if val.len() >= 3 {
                let normal = [val[0], val[1], val[2]];
                normals.push(normal);
            }
        } else if line.starts_with("f ") {
            // Parse face indices
            let indices: Vec<(usize, usize, usize)> = line[2..]
                .split_whitespace()
                .map(|x| {
                    let indices: Vec<usize> = x
                        .split('/')
                        .map(|y| y.parse::<usize>())
                        .collect::<Result<_, _>>()
                        .unwrap();
                    (indices[0], indices[1], indices[2])
                })
                .collect();
        
            if indices.len() >= 3 {
                let v1_index = indices[0].0 - 1;
                let v2_index = indices[1].0 - 1;
                let v3_index = indices[2].0 - 1;
                let normal_index = indices[0].2 - 1;

                let mut rng = rand::thread_rng();
                // let r: f32 = rng.gen_range(0.0..1.0);
                // let g: f32 = rng.gen_range(0.0..1.0);
                // let b: f32 = rng.gen_range(0.0..1.0);
        
                let triangle = Triangle::new(
                    [
                        vertices[v1_index],
                        vertices[v2_index],
                        vertices[v3_index],
                    ],
                    normals[normal_index],
                    0,
                    -1,
                );
                faces.push(triangle);
            }
        }
    }

    Ok(faces)
}

pub fn load_svg(file_path: &str) -> Result<Vec<Vec<[f32; 2]>>, Box<dyn std::error::Error>> {
    let mut file = match File::open(file_path){
        Ok(file) => file,
        Err(e) => panic!("Failed to open SVG: {} | Error: {}", file_path, e),
    };
    let mut svg_content = String::new();
    match file.read_to_string(&mut svg_content){
        Ok(_) => (),
        Err(e) => panic!("Failed to read SVG: {} | Error: {}", file_path, e),
    }

    // Parse the SVG content
    let mut tris = Vec::new();
    let mut height: f32 = 1.0;
    let mut width: f32 = 1.0;

    for line in svg_content.lines() {
        // FIlter for svg size info
        if line.trim().starts_with("<svg ") {
            let width_string = line.split("width=\"").collect::<Vec<&str>>()[1].to_string();
            width = width_string.split("\" ").collect::<Vec<&str>>()[0].to_string().parse::<f32>().unwrap();

            let height_string = line.split("height=\"").collect::<Vec<&str>>()[1].to_string();
            height = height_string.split("\" ").collect::<Vec<&str>>()[0].to_string().parse::<f32>().unwrap();
        // Filter for polygons
        }else if line.trim().starts_with("<polygon") {
            //filter for points
            let mut point_string = line.split("points=\"").collect::<Vec<&str>>()[1].to_string();  //xxxxx points="xxxxx" yyyyy => "xxxxx" yyyyy
            point_string = point_string.split(" \" />").collect::<Vec<&str>>()[0].to_string();      //"xxxxx" yyyyy => "xxxxx"

            //split into single points
            let point_string = point_string.split(" ").collect::<Vec<&str>>();
            let mut points = Vec::new();
            for point in point_string {
                let point = point.split(",").collect::<Vec<&str>>();
                let x = point[0].parse::<f32>().unwrap();
                let y = point[1].parse::<f32>().unwrap();
                points.push([x / width, y / height]);   //scale points to 0.0 - 1.0
            }
            tris.push(points);
        }
    }

    return Ok(tris);
}