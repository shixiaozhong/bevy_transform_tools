use std::io::Cursor;

use crate::mesh::MeshData;

pub fn load_stl_mesh(bytes: &[u8]) -> Result<MeshData, String> {
    let bytes = trim_ascii_stl_prefix(bytes);
    let mut reader = Cursor::new(bytes);
    let mesh =
        stl_io::read_stl(&mut reader).map_err(|error| format!("failed to parse STL: {error}"))?;

    if mesh.faces.is_empty() {
        return Err("STL contains no triangles".to_string());
    }

    let mut positions = Vec::with_capacity(mesh.faces.len() * 3);
    let mut normals = Vec::with_capacity(mesh.faces.len() * 3);
    let mut uvs = Vec::with_capacity(mesh.faces.len() * 3);

    for face in mesh.faces {
        let normal = face.normal.0;
        for vertex_index in face.vertices {
            let vertex = mesh
                .vertices
                .get(vertex_index)
                .ok_or_else(|| format!("STL has an out-of-range vertex index {vertex_index}"))?
                .0;
            positions.push(vertex);
            normals.push(normal);
            uvs.push([0.0, 0.0]);
        }
    }

    Ok(MeshData {
        positions,
        normals,
        uvs,
    })
}

fn trim_ascii_stl_prefix(bytes: &[u8]) -> &[u8] {
    let Some(first_non_whitespace) = bytes.iter().position(|byte| !byte.is_ascii_whitespace())
    else {
        return bytes;
    };
    let trimmed = &bytes[first_non_whitespace..];

    if trimmed.starts_with(b"solid") {
        trimmed
    } else {
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_ascii_stl_triangle() {
        let mesh = load_stl_mesh(
            br#"
            solid test
              facet normal 0 0 1
                outer loop
                  vertex 0 0 0
                  vertex 1 0 0
                  vertex 0 1 0
                endloop
              endfacet
            endsolid test
            "#,
        )
        .unwrap();

        assert_eq!(mesh.positions.len(), 3);
        assert_eq!(mesh.normals[0], [0.0, 0.0, 1.0]);
    }
}
