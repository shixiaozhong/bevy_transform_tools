use std::io::Cursor;

use crate::mesh::MeshData;

pub fn load_obj_mesh(source: &str) -> Result<MeshData, String> {
    let mut reader = Cursor::new(source.as_bytes());
    let (models, _) = tobj::load_obj_buf(&mut reader, &tobj::GPU_LOAD_OPTIONS, |_| {
        Ok((Vec::new(), Default::default()))
    })
    .map_err(|error| format!("failed to parse OBJ: {error}"))?;

    let mut output = MeshData::default();
    for model in models {
        output.extend(model_to_mesh_data(&model)?);
    }

    if output.is_empty() {
        return Err("OBJ contains no renderable triangles".to_string());
    }

    Ok(output)
}

fn model_to_mesh_data(model: &tobj::Model) -> Result<MeshData, String> {
    let mesh = &model.mesh;
    let mut positions = Vec::with_capacity(mesh.indices.len());
    let mut normals = Vec::with_capacity(mesh.indices.len());
    let mut uvs = Vec::with_capacity(mesh.indices.len());

    for &index in &mesh.indices {
        let index = index as usize;
        positions.push(read_position(&mesh.positions, index, &model.name)?);
        normals.push(read_normal(&mesh.normals, index).unwrap_or([0.0, 0.0, 0.0]));
        uvs.push(read_uv(&mesh.texcoords, index).unwrap_or([0.0, 0.0]));
    }

    Ok(MeshData {
        positions,
        normals,
        uvs,
    })
}

fn read_position(values: &[f32], index: usize, model_name: &str) -> Result<[f32; 3], String> {
    let offset = index * 3;
    if offset + 2 >= values.len() {
        return Err(format!(
            "OBJ model '{model_name}' has an out-of-range vertex index {index}"
        ));
    }

    Ok([values[offset], values[offset + 1], values[offset + 2]])
}

fn read_normal(values: &[f32], index: usize) -> Option<[f32; 3]> {
    let offset = index * 3;
    (offset + 2 < values.len()).then(|| [values[offset], values[offset + 1], values[offset + 2]])
}

fn read_uv(values: &[f32], index: usize) -> Option<[f32; 2]> {
    let offset = index * 2;
    (offset + 1 < values.len()).then(|| [values[offset], values[offset + 1]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_obj_quad_as_two_triangles() {
        let mesh = load_obj_mesh(
            r#"
            v 0 0 0
            v 1 0 0
            v 1 1 0
            v 0 1 0
            f 1 2 3 4
            "#,
        )
        .unwrap();

        assert_eq!(mesh.positions.len(), 6);
    }
}
