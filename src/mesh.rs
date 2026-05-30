use bevy::{asset::RenderAssetUsages, prelude::*, render::render_resource::PrimitiveTopology};

#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
}

#[derive(Clone, Copy, Debug)]
pub struct MeshBounds {
    pub center: Vec3,
    pub size: Vec3,
}

impl MeshData {
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn extend(&mut self, other: MeshData) {
        self.positions.extend(other.positions);
        self.normals.extend(other.normals);
        self.uvs.extend(other.uvs);
    }

    pub fn bounds(&self) -> Option<MeshBounds> {
        let first = Vec3::from_array(*self.positions.first()?);
        let mut min = first;
        let mut max = first;

        for position in &self.positions[1..] {
            let position = Vec3::from_array(*position);
            min = min.min(position);
            max = max.max(position);
        }

        Some(MeshBounds {
            center: (min + max) * 0.5,
            size: max - min,
        })
    }

    pub fn volume(&self) -> f64 {
        let mut volume = 0.0_f64;
        let mut compensation = 0.0_f64;

        for tri in (0..self.positions.len()).step_by(3) {
            if tri + 2 >= self.positions.len() {
                break;
            }

            let a = self.positions[tri].map(f64::from);
            let b = self.positions[tri + 1].map(f64::from);
            let c = self.positions[tri + 2].map(f64::from);
            let cross = [
                b[1] * c[2] - b[2] * c[1],
                b[2] * c[0] - b[0] * c[2],
                b[0] * c[1] - b[1] * c[0],
            ];
            let tetra_volume = (a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2]) / 6.0;
            let corrected = tetra_volume - compensation;
            let next = volume + corrected;
            compensation = (next - volume) - corrected;
            volume = next;
        }

        volume.abs()
    }

    pub fn into_mesh(mut self) -> Mesh {
        self.fill_missing_attributes();

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh
    }

    fn fill_missing_attributes(&mut self) {
        if self.normals.len() != self.positions.len() {
            self.normals = vec![[0.0, 0.0, 0.0]; self.positions.len()];
        }

        for tri in (0..self.positions.len()).step_by(3) {
            if tri + 2 >= self.positions.len() {
                break;
            }

            let has_missing = self.normals[tri..=tri + 2]
                .iter()
                .any(|normal| Vec3::from_array(*normal).length_squared() < f32::EPSILON);

            if has_missing {
                let a = Vec3::from_array(self.positions[tri]);
                let b = Vec3::from_array(self.positions[tri + 1]);
                let c = Vec3::from_array(self.positions[tri + 2]);
                let normal = (b - a).cross(c - a).normalize_or_zero().to_array();
                self.normals[tri] = normal;
                self.normals[tri + 1] = normal;
                self.normals[tri + 2] = normal;
            }
        }

        if self.uvs.len() != self.positions.len() {
            self.uvs = vec![[0.0, 0.0]; self.positions.len()];
        }
    }
}
