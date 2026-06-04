use std::fmt;

pub(crate) type Point2 = [f64; 2];
pub(crate) type Triangle2 = [Point2; 3];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TriangulationError {
    NoPolygons,
    Failed(String),
}

impl fmt::Display for TriangulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPolygons => write!(f, "triangulation input contained no polygons"),
            Self::Failed(error) => write!(f, "triangulation failed: {error}"),
        }
    }
}

impl std::error::Error for TriangulationError {}

pub(crate) fn triangulate_paths(
    paths: &[Vec<Point2>],
) -> Result<Vec<Triangle2>, TriangulationError> {
    let polygons = group_paths(paths)?;
    let mut triangles = Vec::new();

    for polygon in polygons {
        triangles.extend(triangulate_polygon(&polygon)?);
    }

    if triangles.is_empty() {
        Err(TriangulationError::NoPolygons)
    } else {
        Ok(triangles)
    }
}

#[derive(Clone, Debug)]
struct PolygonWithHoles {
    contour: Vec<Point2>,
    holes: Vec<Vec<Point2>>,
}

fn group_paths(paths: &[Vec<Point2>]) -> Result<Vec<PolygonWithHoles>, TriangulationError> {
    let paths = paths
        .iter()
        .filter_map(|path| {
            let cleaned = clean_path(path);
            (cleaned.len() >= 3 && signed_area(&cleaned).abs() > 1.0e-12).then_some(cleaned)
        })
        .collect::<Vec<_>>();

    if paths.is_empty() {
        return Err(TriangulationError::NoPolygons);
    }

    let mut polygons = Vec::<PolygonWithHoles>::new();
    let mut holes = Vec::<Vec<Point2>>::new();

    for path in paths {
        if signed_area(&path) >= 0.0 {
            polygons.push(PolygonWithHoles {
                contour: path,
                holes: Vec::new(),
            });
        } else {
            holes.push(path);
        }
    }

    if polygons.is_empty() {
        polygons.extend(holes.drain(..).map(|mut contour| {
            contour.reverse();
            PolygonWithHoles {
                contour,
                holes: Vec::new(),
            }
        }));
        return Ok(polygons);
    }

    for hole in holes {
        let Some(index) = containing_polygon_index(&hole, &polygons) else {
            continue;
        };
        polygons[index].holes.push(hole);
    }

    Ok(polygons)
}

fn containing_polygon_index(hole: &[Point2], polygons: &[PolygonWithHoles]) -> Option<usize> {
    let point = hole[0];
    polygons
        .iter()
        .enumerate()
        .filter(|(_, polygon)| point_in_polygon(point, &polygon.contour))
        .min_by(|(_, a), (_, b)| {
            signed_area(&a.contour)
                .abs()
                .partial_cmp(&signed_area(&b.contour).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, _)| index)
}

fn triangulate_polygon(polygon: &PolygonWithHoles) -> Result<Vec<Triangle2>, TriangulationError> {
    let mut vertices = Vec::<f64>::new();
    let mut point_lookup = Vec::<Point2>::new();
    let mut hole_indices = Vec::<usize>::new();

    append_path(&polygon.contour, &mut vertices, &mut point_lookup);
    for hole in &polygon.holes {
        hole_indices.push(point_lookup.len());
        append_path(hole, &mut vertices, &mut point_lookup);
    }

    let indices = earcutr::earcut(&vertices, &hole_indices, 2)
        .map_err(|error| TriangulationError::Failed(format!("{error:?}")))?;

    if indices.len() % 3 != 0 {
        return Err(TriangulationError::Failed(
            "earcut returned non-triangle index count".to_string(),
        ));
    }

    indices
        .chunks_exact(3)
        .map(|chunk| {
            let a = *point_lookup
                .get(chunk[0])
                .ok_or_else(|| TriangulationError::Failed("triangle index out of range".into()))?;
            let b = *point_lookup
                .get(chunk[1])
                .ok_or_else(|| TriangulationError::Failed("triangle index out of range".into()))?;
            let c = *point_lookup
                .get(chunk[2])
                .ok_or_else(|| TriangulationError::Failed("triangle index out of range".into()))?;
            Ok([a, b, c])
        })
        .collect()
}

fn append_path(path: &[Point2], vertices: &mut Vec<f64>, point_lookup: &mut Vec<Point2>) {
    for point in path {
        vertices.push(point[0]);
        vertices.push(point[1]);
        point_lookup.push(*point);
    }
}

fn clean_path(path: &[Point2]) -> Vec<Point2> {
    let mut points = Vec::with_capacity(path.len());
    for point in path {
        if points
            .last()
            .is_none_or(|last| squared_distance(*last, *point) > 1.0e-18)
        {
            points.push(*point);
        }
    }
    if points.len() > 1 && squared_distance(points[0], *points.last().unwrap()) <= 1.0e-18 {
        points.pop();
    }
    points
}

fn signed_area(points: &[Point2]) -> f64 {
    let mut area = 0.0;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        area += a[0] * b[1] - b[0] * a[1];
    }
    area * 0.5
}

fn point_in_polygon(point: Point2, polygon: &[Point2]) -> bool {
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

fn squared_distance(a: Point2, b: Point2) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangulates_square() {
        let triangles =
            triangulate_paths(&[vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]]).unwrap();

        assert_eq!(triangles.len(), 2);
    }

    #[test]
    fn triangulates_square_with_hole() {
        let triangles = triangulate_paths(&[
            vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]],
            vec![[1.0, 1.0], [1.0, 3.0], [3.0, 3.0], [3.0, 1.0]],
        ])
        .unwrap();

        assert!(!triangles.is_empty());
    }
}
