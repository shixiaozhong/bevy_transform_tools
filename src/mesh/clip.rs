use std::collections::{HashMap, HashSet};
use std::fmt;

use bevy::prelude::*;

use super::{
    MeshData,
    triangulation::{TriangulationError, triangulate_paths},
};

const BASE_EPSILON: f32 = 1.0e-5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CutAxis {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CutKeep {
    Upper,
    Lower,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CutPlane {
    pub(crate) axis: CutAxis,
    pub(crate) position: f32,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct MeshCutResult {
    pub(crate) upper: Option<MeshData>,
    pub(crate) lower: Option<MeshData>,
    pub(crate) warnings: Vec<CutWarning>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CutWarning {
    OpenSection,
    CapTriangulationFailed(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CutError {
    EmptyMesh,
    InvalidMesh,
}

impl fmt::Display for CutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMesh => write!(f, "mesh contains no triangles"),
            Self::InvalidMesh => write!(f, "mesh has invalid triangle data"),
        }
    }
}

impl std::error::Error for CutError {}

#[derive(Clone, Copy)]
struct VertexData {
    position: Vec3,
    normal: Vec3,
    uv: Vec2,
    distance: f32,
}

#[derive(Clone, Copy)]
struct SectionSegment {
    a: Vec2,
    b: Vec2,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct PointKey {
    x: i64,
    y: i64,
}

pub(crate) fn cut_mesh(
    mesh: &MeshData,
    plane: CutPlane,
    cap: bool,
) -> Result<MeshCutResult, CutError> {
    if mesh.positions.is_empty() {
        return Err(CutError::EmptyMesh);
    }
    if mesh.positions.len() % 3 != 0 {
        return Err(CutError::InvalidMesh);
    }

    let epsilon = mesh_epsilon(mesh);
    let mut upper = MeshData::default();
    let mut lower = MeshData::default();
    let mut section_segments = Vec::new();

    for tri in (0..mesh.positions.len()).step_by(3) {
        let triangle = [
            vertex_data(mesh, tri, plane),
            vertex_data(mesh, tri + 1, plane),
            vertex_data(mesh, tri + 2, plane),
        ];

        append_clipped_triangle(&mut upper, &triangle, true, epsilon);
        append_clipped_triangle(&mut lower, &triangle, false, epsilon);

        if let Some(segment) = section_segment(&triangle, plane, epsilon) {
            section_segments.push(segment);
        }
    }

    let mut warnings = Vec::new();
    if cap && !section_segments.is_empty() {
        let mut upper_loops = plane_boundary_loops(&upper, plane, epsilon, &mut warnings);
        let mut lower_loops = plane_boundary_loops(&lower, plane, epsilon, &mut warnings);

        if upper_loops.is_empty() || lower_loops.is_empty() {
            let fallback_loops = section_loops(&section_segments, epsilon, &mut warnings);
            if upper_loops.is_empty() {
                upper_loops = fallback_loops.clone();
            }
            if lower_loops.is_empty() {
                lower_loops = fallback_loops;
            }
        }

        if !upper_loops.is_empty() {
            append_caps(&mut upper, &upper_loops, plane, true, &mut warnings);
        }
        if !lower_loops.is_empty() {
            append_caps(&mut lower, &lower_loops, plane, false, &mut warnings);
        }
    }

    let upper = (!upper.positions.is_empty()).then_some(upper);
    let lower = (!lower.positions.is_empty()).then_some(lower);

    Ok(MeshCutResult {
        upper,
        lower,
        warnings,
    })
}

fn mesh_epsilon(mesh: &MeshData) -> f32 {
    mesh.bounds()
        .map(|bounds| bounds.size.max_element().abs().max(1.0) * BASE_EPSILON)
        .unwrap_or(BASE_EPSILON)
}

fn vertex_data(mesh: &MeshData, index: usize, plane: CutPlane) -> VertexData {
    let position = Vec3::from_array(mesh.positions[index]);
    let normal = mesh
        .normals
        .get(index)
        .copied()
        .map(Vec3::from_array)
        .filter(|normal| normal.length_squared() > f32::EPSILON)
        .unwrap_or(Vec3::ZERO);
    let uv = mesh
        .uvs
        .get(index)
        .copied()
        .map(Vec2::from_array)
        .unwrap_or(Vec2::ZERO);
    VertexData {
        position,
        normal,
        uv,
        distance: axis_value(position, plane.axis) - plane.position,
    }
}

fn append_clipped_triangle(
    output: &mut MeshData,
    triangle: &[VertexData; 3],
    keep_upper: bool,
    epsilon: f32,
) {
    let clipped = clip_polygon(triangle, keep_upper, epsilon);
    if clipped.len() < 3 {
        return;
    }

    for i in 1..clipped.len() - 1 {
        append_triangle(output, clipped[0], clipped[i], clipped[i + 1]);
    }
}

fn clip_polygon(triangle: &[VertexData; 3], keep_upper: bool, epsilon: f32) -> Vec<VertexData> {
    let mut output = triangle.to_vec();
    let input = output;
    output = Vec::new();

    for i in 0..input.len() {
        let previous = input[(i + input.len() - 1) % input.len()];
        let current = input[i];
        let previous_inside = is_inside(previous.distance, keep_upper, epsilon);
        let current_inside = is_inside(current.distance, keep_upper, epsilon);

        if current_inside {
            if !previous_inside {
                output.push(interpolate_at_plane(previous, current));
            }
            output.push(clamp_to_plane(current, keep_upper, epsilon));
        } else if previous_inside {
            output.push(interpolate_at_plane(previous, current));
        }
    }

    dedupe_vertices(output, epsilon)
}

fn is_inside(distance: f32, keep_upper: bool, epsilon: f32) -> bool {
    if keep_upper {
        distance >= -epsilon
    } else {
        distance <= epsilon
    }
}

fn clamp_to_plane(mut vertex: VertexData, keep_upper: bool, epsilon: f32) -> VertexData {
    if vertex.distance.abs() <= epsilon {
        vertex.distance = 0.0;
    } else if keep_upper && vertex.distance < 0.0 {
        vertex.distance = 0.0;
    } else if !keep_upper && vertex.distance > 0.0 {
        vertex.distance = 0.0;
    }
    vertex
}

fn interpolate_at_plane(a: VertexData, b: VertexData) -> VertexData {
    let denominator = a.distance - b.distance;
    let t = if denominator.abs() > f32::EPSILON {
        a.distance / denominator
    } else {
        0.5
    }
    .clamp(0.0, 1.0);
    let normal = a.normal.lerp(b.normal, t).normalize_or_zero();
    VertexData {
        position: a.position.lerp(b.position, t),
        normal,
        uv: a.uv.lerp(b.uv, t),
        distance: 0.0,
    }
}

fn dedupe_vertices(vertices: Vec<VertexData>, epsilon: f32) -> Vec<VertexData> {
    let mut deduped = Vec::with_capacity(vertices.len());
    for vertex in vertices {
        if deduped.last().is_none_or(|last: &VertexData| {
            last.position.distance_squared(vertex.position) > epsilon * epsilon
        }) {
            deduped.push(vertex);
        }
    }
    if deduped.len() > 1
        && deduped[0]
            .position
            .distance_squared(deduped.last().unwrap().position)
            <= epsilon * epsilon
    {
        deduped.pop();
    }
    deduped
}

fn append_triangle(output: &mut MeshData, a: VertexData, b: VertexData, c: VertexData) {
    let area = (b.position - a.position)
        .cross(c.position - a.position)
        .length_squared();
    if area <= f32::EPSILON {
        return;
    }

    output.positions.push(a.position.to_array());
    output.positions.push(b.position.to_array());
    output.positions.push(c.position.to_array());
    output.normals.push(a.normal.to_array());
    output.normals.push(b.normal.to_array());
    output.normals.push(c.normal.to_array());
    output.uvs.push(a.uv.to_array());
    output.uvs.push(b.uv.to_array());
    output.uvs.push(c.uv.to_array());
}

fn section_segment(
    triangle: &[VertexData; 3],
    plane: CutPlane,
    epsilon: f32,
) -> Option<SectionSegment> {
    let mut points = Vec::with_capacity(3);
    for vertex in triangle {
        if vertex.distance.abs() <= epsilon {
            points.push(vertex.position);
        }
    }
    for edge in [(0, 1), (1, 2), (2, 0)] {
        let a = triangle[edge.0];
        let b = triangle[edge.1];
        if (a.distance < -epsilon && b.distance > epsilon)
            || (a.distance > epsilon && b.distance < -epsilon)
        {
            points.push(interpolate_at_plane(a, b).position);
        }
    }

    let points = dedupe_points(points, epsilon);
    if points.len() == 2 {
        Some(SectionSegment {
            a: project_point(points[0], plane.axis),
            b: project_point(points[1], plane.axis),
        })
    } else {
        None
    }
}

fn dedupe_points(points: Vec<Vec3>, epsilon: f32) -> Vec<Vec3> {
    let mut deduped = Vec::with_capacity(points.len());
    for point in points {
        if !deduped
            .iter()
            .any(|existing: &Vec3| existing.distance_squared(point) <= epsilon * epsilon)
        {
            deduped.push(point);
        }
    }
    deduped
}

fn section_loops(
    segments: &[SectionSegment],
    epsilon: f32,
    warnings: &mut Vec<CutWarning>,
) -> Vec<Vec<[f64; 2]>> {
    let quantize = Quantizer::new(epsilon);
    let mut points = HashMap::<PointKey, Vec2>::new();
    let mut adjacency = HashMap::<PointKey, Vec<PointKey>>::new();
    let mut edges = Vec::<(PointKey, PointKey)>::new();
    let mut seen = HashSet::<(PointKey, PointKey)>::new();

    for segment in segments {
        let a = quantize.key(segment.a);
        let b = quantize.key(segment.b);
        if a == b {
            continue;
        }
        let edge_key = sorted_edge(a, b);
        if !seen.insert(edge_key) {
            continue;
        }
        points.entry(a).or_insert(segment.a);
        points.entry(b).or_insert(segment.b);
        adjacency.entry(a).or_default().push(b);
        adjacency.entry(b).or_default().push(a);
        edges.push((a, b));
    }

    let mut used = HashSet::<(PointKey, PointKey)>::new();
    let mut loops = Vec::new();
    for (start, first_next) in edges {
        if used.contains(&sorted_edge(start, first_next)) {
            continue;
        }

        let mut path = vec![*points.get(&start).unwrap()];
        let mut previous = start;
        let mut current = first_next;
        used.insert(sorted_edge(previous, current));
        let mut closed = false;

        for _ in 0..points.len() + 1 {
            path.push(*points.get(&current).unwrap());
            if current == start {
                closed = true;
                break;
            }

            let Some(neighbors) = adjacency.get(&current) else {
                break;
            };
            let next = neighbors.iter().copied().find(|candidate| {
                *candidate != previous && !used.contains(&sorted_edge(current, *candidate))
            });
            let Some(next) = next else {
                break;
            };

            previous = current;
            current = next;
            used.insert(sorted_edge(previous, current));
        }

        if closed {
            path.pop();
            if path.len() >= 3 {
                loops.push(path.into_iter().map(|p| [p.x as f64, p.y as f64]).collect());
            }
        } else {
            warnings.push(CutWarning::OpenSection);
            if path.len() >= 3 {
                loops.push(path.into_iter().map(|p| [p.x as f64, p.y as f64]).collect());
            }
        }
    }

    orient_loops(loops)
}

fn orient_loops(mut loops: Vec<Vec<[f64; 2]>>) -> Vec<Vec<[f64; 2]>> {
    let depths = loops
        .iter()
        .map(|loop_points| {
            let point = loop_points[0];
            loops
                .iter()
                .filter(|candidate| !std::ptr::eq(*candidate, loop_points))
                .filter(|candidate| point_in_polygon(point, candidate))
                .count()
        })
        .collect::<Vec<_>>();

    for (loop_points, depth) in loops.iter_mut().zip(depths) {
        let should_be_ccw = depth % 2 == 0;
        let is_ccw = signed_area(loop_points) > 0.0;
        if should_be_ccw != is_ccw {
            loop_points.reverse();
        }
    }

    loops
}

fn append_caps(
    mesh: &mut MeshData,
    loops: &[Vec<[f64; 2]>],
    plane: CutPlane,
    upper: bool,
    warnings: &mut Vec<CutWarning>,
) {
    let triangles = match triangulate_paths(loops) {
        Ok(triangles) => triangles,
        Err(error) => {
            warnings.push(CutWarning::CapTriangulationFailed(error.to_string()));
            return;
        }
    };

    let desired_normal = if upper {
        -axis_vector(plane.axis)
    } else {
        axis_vector(plane.axis)
    };
    for triangle in triangles {
        let mut a = unproject_point(triangle[0], plane);
        let mut b = unproject_point(triangle[1], plane);
        let c = unproject_point(triangle[2], plane);
        if (b - a).cross(c - a).dot(desired_normal) < 0.0 {
            std::mem::swap(&mut a, &mut b);
        }
        append_triangle(
            mesh,
            cap_vertex(a, desired_normal),
            cap_vertex(b, desired_normal),
            cap_vertex(c, desired_normal),
        );
    }
}

fn plane_boundary_loops(
    mesh: &MeshData,
    plane: CutPlane,
    epsilon: f32,
    warnings: &mut Vec<CutWarning>,
) -> Vec<Vec<[f64; 2]>> {
    let quantize = Quantizer::new(epsilon);
    let mut edges = HashMap::<(PointKey, PointKey), (SectionSegment, usize)>::new();

    for triangle in mesh.positions.chunks_exact(3) {
        for (a_index, b_index) in [(0, 1), (1, 2), (2, 0)] {
            let a = Vec3::from_array(triangle[a_index]);
            let b = Vec3::from_array(triangle[b_index]);
            if !point_is_on_plane(a, plane, epsilon) || !point_is_on_plane(b, plane, epsilon) {
                continue;
            }

            let segment = SectionSegment {
                a: project_point(a, plane.axis),
                b: project_point(b, plane.axis),
            };
            let a_key = quantize.key(segment.a);
            let b_key = quantize.key(segment.b);
            if a_key == b_key {
                continue;
            }

            let key = sorted_edge(a_key, b_key);
            edges
                .entry(key)
                .and_modify(|(_, count)| *count += 1)
                .or_insert((segment, 1));
        }
    }

    let segments = edges
        .into_values()
        .filter_map(|(segment, count)| (count == 1).then_some(segment))
        .collect::<Vec<_>>();
    if segments.is_empty() {
        Vec::new()
    } else {
        section_loops(&segments, epsilon, warnings)
    }
}

fn cap_vertex(position: Vec3, normal: Vec3) -> VertexData {
    VertexData {
        position,
        normal,
        uv: Vec2::ZERO,
        distance: 0.0,
    }
}

fn point_is_on_plane(point: Vec3, plane: CutPlane, epsilon: f32) -> bool {
    (axis_value(point, plane.axis) - plane.position).abs() <= epsilon
}

fn axis_value(point: Vec3, axis: CutAxis) -> f32 {
    match axis {
        CutAxis::X => point.x,
        CutAxis::Y => point.y,
        CutAxis::Z => point.z,
    }
}

fn axis_vector(axis: CutAxis) -> Vec3 {
    match axis {
        CutAxis::X => Vec3::X,
        CutAxis::Y => Vec3::Y,
        CutAxis::Z => Vec3::Z,
    }
}

fn project_point(point: Vec3, axis: CutAxis) -> Vec2 {
    match axis {
        CutAxis::X => Vec2::new(point.y, point.z),
        CutAxis::Y => Vec2::new(point.x, point.z),
        CutAxis::Z => Vec2::new(point.x, point.y),
    }
}

fn unproject_point(point: [f64; 2], plane: CutPlane) -> Vec3 {
    let a = point[0] as f32;
    let b = point[1] as f32;
    match plane.axis {
        CutAxis::X => Vec3::new(plane.position, a, b),
        CutAxis::Y => Vec3::new(a, plane.position, b),
        CutAxis::Z => Vec3::new(a, b, plane.position),
    }
}

fn sorted_edge(a: PointKey, b: PointKey) -> (PointKey, PointKey) {
    if (a.x, a.y) <= (b.x, b.y) {
        (a, b)
    } else {
        (b, a)
    }
}

struct Quantizer {
    scale: f32,
}

impl Quantizer {
    fn new(epsilon: f32) -> Self {
        Self {
            scale: 1.0 / epsilon.max(1.0e-9),
        }
    }

    fn key(&self, point: Vec2) -> PointKey {
        PointKey {
            x: (point.x * self.scale).round() as i64,
            y: (point.y * self.scale).round() as i64,
        }
    }
}

fn signed_area(points: &[[f64; 2]]) -> f64 {
    let mut area = 0.0;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        area += a[0] * b[1] - b[0] * a[1];
    }
    area * 0.5
}

fn point_in_polygon(point: [f64; 2], polygon: &[[f64; 2]]) -> bool {
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[j];
        if ((a[1] > point[1]) != (b[1] > point[1]))
            && (point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0])
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

impl From<TriangulationError> for CutWarning {
    fn from(error: TriangulationError) -> Self {
        Self::CapTriangulationFailed(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_mesh() -> MeshData {
        let p = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ];
        let triangles = [
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];
        let mut mesh = MeshData::default();
        for triangle in triangles {
            for index in triangle {
                mesh.positions.push(p[index]);
            }
        }
        mesh
    }

    fn square_tube_side_mesh() -> MeshData {
        let mut mesh = MeshData::default();
        let outer = [
            Vec2::new(-2.0, -2.0),
            Vec2::new(2.0, -2.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(-2.0, 2.0),
        ];
        let inner = [
            Vec2::new(-1.0, -1.0),
            Vec2::new(-1.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, -1.0),
        ];

        append_square_tube_walls(&mut mesh, &outer);
        append_square_tube_walls(&mut mesh, &inner);
        mesh
    }

    fn append_square_tube_walls(mesh: &mut MeshData, loop_points: &[Vec2; 4]) {
        for index in 0..loop_points.len() {
            let a = loop_points[index];
            let b = loop_points[(index + 1) % loop_points.len()];
            let bottom_a = Vec3::new(a.x, a.y, 0.0);
            let bottom_b = Vec3::new(b.x, b.y, 0.0);
            let top_b = Vec3::new(b.x, b.y, 1.0);
            let top_a = Vec3::new(a.x, a.y, 1.0);
            append_test_triangle(mesh, bottom_a, bottom_b, top_b);
            append_test_triangle(mesh, bottom_a, top_b, top_a);
        }
    }

    fn append_test_triangle(mesh: &mut MeshData, a: Vec3, b: Vec3, c: Vec3) {
        mesh.positions.push(a.to_array());
        mesh.positions.push(b.to_array());
        mesh.positions.push(c.to_array());
    }

    fn open_box_side_mesh() -> MeshData {
        let p = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
        ];
        let side_faces = [[0, 1, 5, 4], [1, 2, 6, 5], [2, 3, 7, 6]];
        let mut mesh = MeshData::default();
        for face in side_faces {
            append_test_triangle(&mut mesh, p[face[0]], p[face[1]], p[face[2]]);
            append_test_triangle(&mut mesh, p[face[0]], p[face[2]], p[face[3]]);
        }
        mesh
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn cuts_cube_into_capped_halves() {
        let result = cut_mesh(
            &cube_mesh(),
            CutPlane {
                axis: CutAxis::Z,
                position: 0.5,
            },
            true,
        )
        .unwrap();

        assert!(result.warnings.is_empty(), "{:?}", result.warnings);
        let upper = result.upper.unwrap();
        let lower = result.lower.unwrap();
        let upper_bounds = upper.bounds().unwrap();
        let lower_bounds = lower.bounds().unwrap();
        assert_close(upper_bounds.size.z as f64, 0.5);
        assert_close(lower_bounds.size.z as f64, 0.5);
        assert_close(upper.volume(), 0.5);
        assert_close(lower.volume(), 0.5);
    }

    #[test]
    fn cuts_square_tube_with_capped_hole() {
        let result = cut_mesh(
            &square_tube_side_mesh(),
            CutPlane {
                axis: CutAxis::Z,
                position: 0.5,
            },
            true,
        )
        .unwrap();

        assert!(result.warnings.is_empty(), "{:?}", result.warnings);
        let upper = result.upper.unwrap();
        let cap_triangles = upper
            .positions
            .chunks_exact(3)
            .filter(|triangle| {
                triangle
                    .iter()
                    .all(|point| (point[2] - 0.5).abs() < BASE_EPSILON)
            })
            .count();
        assert!(cap_triangles >= 8, "expected ring cap, got {cap_triangles}");
    }

    #[test]
    fn cuts_open_section_with_best_effort_cap() {
        let result = cut_mesh(
            &open_box_side_mesh(),
            CutPlane {
                axis: CutAxis::Z,
                position: 0.5,
            },
            true,
        )
        .unwrap();

        assert!(
            result.warnings.contains(&CutWarning::OpenSection),
            "{:?}",
            result.warnings
        );
        let upper = result.upper.unwrap();
        let cap_triangles = upper
            .positions
            .chunks_exact(3)
            .filter(|triangle| {
                triangle
                    .iter()
                    .all(|point| (point[2] - 0.5).abs() < BASE_EPSILON)
            })
            .count();
        assert!(cap_triangles > 0, "expected best-effort cap");
    }
}
