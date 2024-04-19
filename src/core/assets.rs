use crate::core::model;
use crate::core::renderer::Vertex;
use crate::core::texture;


use std::io::{BufReader, Cursor};
use cfg_if::cfg_if;




#[cfg(target_arch = "wasm32")]
fn format_url(file_name: &str) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let mut origin = location.origin().unwrap();
    if !origin.ends_with("f✦stop") {
        origin = format!("{}/assets", origin);
    }
    let base = reqwest::Url::parse(&format!("{}/", origin,)).unwrap();
    base.join(file_name).unwrap()
}




pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    println!("Loading {:?}", file_name);
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            let url = format_url(file_name);
            let txt = reqwest::get(url)
                .await?
                .text()
                .await?;
        } else {
            let path = std::path::Path::new(env!("OUT_DIR"))
                .join("assets")
                .join(file_name);
            let txt = std::fs::read_to_string(path)?;
        }
    }
    Ok(txt)
}




pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    println!("Loading {:?}", file_name);
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            let url = format_url(file_name);
            let data = reqwest::get(url)
                .await?
                .bytes()
                .await?
                .to_vec();
        } else {
            let path = std::path::Path::new(env!("OUT_DIR"))
                .join("assets")
                .join(file_name);
            let data = std::fs::read(path)?;
        }
    }
    Ok(data)
}




pub async fn load_texture(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<texture::Texture> 
{
    if file_name.is_empty() {
        return Ok(texture::Texture::default_white(device,queue));
    }
    let data = load_binary(file_name).await?;
    texture::Texture::from_bytes(device, queue, &data, file_name)
}






pub async fn load_model(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<(Vec<model::Mesh>, Vec<model::Material>)> 
{
    let obj_text = load_string(file_name).await?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);

    let (models, obj_materials) = tobj::load_obj_buf_async(
        &mut obj_reader,
        &tobj::LoadOptions 
        {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move 
        {
            let mat_text = load_string(&p).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        }
    ) .await?;


    let mut materials = Vec::new();
    
    if obj_materials.is_ok() 
    {
        for m in obj_materials? 
        {
            let diffuse_texture = load_texture(&m.diffuse_texture, device, queue).await?;
            let texture = diffuse_texture.view().unwrap();
            let diffuse_color = model::Color { color: [m.diffuse[0], m.diffuse[1], m.diffuse[2], 1.0] };
            materials.push(
                model::Material 
                {
                    name: m.name,
                    diffuse_texture: texture,
                    diffuse_color: model::Color {color : [m.diffuse[0], m.diffuse[1], m.diffuse[2], 1.0]},
                }
            );
        }
    }

    let meshes = models
        .into_iter()
        .map(|m| 
            {
                let pos = (0..m.mesh.positions.len() / 3)
                    .map(|i| [
                            m.mesh.positions[i * 3],
                            m.mesh.positions[i * 3 + 1],
                            m.mesh.positions[i * 3 + 2],
                        ]
                    );

                let uv : Vec<[f32; 2]> = if m.mesh.texcoords.len() > 0 {
                    (0..m.mesh.texcoords.len() / 2)
                        .map(|i| [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]])
                        .collect()
                } else {
                    (0..m.mesh.positions.len() / 3)
                        .map(|_| [0.0, 0.0])
                        .collect()
                }; 

                let normals : Vec<[f32; 3]> = if m.mesh.normals.len() > 0 {
                    (0..m.mesh.normals.len() / 3)
                        .map(|i| [
                            m.mesh.normals[i * 3],
                            m.mesh.normals[i * 3 + 1],
                            m.mesh.normals[i * 3 + 2],
                        ])
                        .collect()
                } else {
                    (0..m.mesh.positions.len() / 3)
                        .map(|_| [0.0, 0.0, 0.0])
                        .collect()
                };

                let vertices = pos.zip(uv).zip(normals).map(|((pos, uv), normal)| Vertex {
                    position: pos,
                    uv,
                    normal,
                }).collect::<Vec<_>>();

                let indices = m.mesh.indices.clone();


                model::Mesh {
                    name: file_name.to_string(),
                    vertices,
                    indices,
                    num_elements: m.mesh.indices.len() as u32,
                }
        })
        .collect::<Vec<_>>();
    Ok((meshes, materials))
}

