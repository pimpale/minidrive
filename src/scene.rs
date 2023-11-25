use crate::vertex::mVertex as Vertex;
use cgmath::{InnerSpace, Vector3};

pub struct Scene {
    objects: Vec<Object>,
}

impl Scene {
    pub fn new(objects: Vec<Object>) -> Scene {
        Scene { objects }
    }

    pub fn vertexes(&self) -> Vec<Vertex> {
        let mut vertexes = Vec::new();
        for object in &self.objects {
            vertexes.extend(object.vertexes.clone());
        }
        vertexes
    }
}

pub struct Object {
    vertexes: Vec<Vertex>,
}

impl Object {
    pub fn new(vertexes: Vec<Vertex>) -> Object {
        Object { vertexes }
    }

    pub fn cube(x: i32, y: i32, z: i32) -> Object {
        let lbu = Vertex {
            loc: [x as f32, y as f32, z as f32],
            color: [0.5, 0.9, 0.5, 1.0],
        };
        let rbu = Vertex {
            loc: [(x + 1) as f32, y as f32, z as f32],
            color: [0.5, 0.5, 0.9, 1.0],
        };
        let lfu = Vertex {
            loc: [x as f32, y as f32, (z + 1) as f32],
            color: [0.9, 0.5, 0.5, 1.0],
        };
        let rfu = Vertex {
            loc: [(x + 1) as f32, y as f32, (z + 1) as f32],
            color: [0.5, 0.9, 0.5, 1.0],
        };
        let lbl = Vertex {
            loc: [x as f32, (y + 1) as f32, z as f32],
            color: [0.5, 0.5, 0.9, 1.0],
        };
        let rbl = Vertex {
            loc: [(x + 1) as f32, (y + 1) as f32, z as f32],
            color: [0.9, 0.5, 0.5, 1.0],
        };
        let lfl = Vertex {
            loc: [x as f32, (y + 1) as f32, (z + 1) as f32],
            color: [0.5, 0.5, 0.5, 1.0],
        };
        let rfl = Vertex {
            loc: [(x + 1) as f32, (y + 1) as f32, (z + 1) as f32],
            color: [0.5, 0.5, 0.5, 1.0],
        };

        let vertexes = vec![
            lbu, rbu, lfu, lfu, rfu, rbu, // upper square
            lbl, rbl, lfl, lfl, rfl, rbl, // lower square
            lfu, rfu, lfl, lfl, rfl, rfu, // front square
            lbu, rbu, lbl, lbl, rbl, rbu, // back square
            lbu, lfu, lbl, lbl, lfl, lfu, // left square
            rbu, rfu, rbl, rbl, rfl, rfu, // right square
        ];

        Object::new(vertexes)
    }

    pub fn flat_polyline(points: Vec<[f32; 3]>, width: f32) -> Object {
        let points: Vec<Vector3<f32>> = points
            .iter()
            .map(|p| Vector3::new(p[0], p[1], p[2]))
            .collect();
        let normals: Vec<Vector3<f32>> = std::iter::repeat(Vector3::unit_y())
            .take(points.len())
            .collect();
        let width: Vec<f32> = std::iter::repeat(width).take(points.len()).collect();
        Object::polyline(points, normals, width)
    }

    fn polyline(points: Vec<Vector3<f32>>, normals: Vec<Vector3<f32>>, width: Vec<f32>) -> Object {
        assert!(points.len() > 1, "not enough points");
        assert!(
            points.len() == normals.len(),
            "there must be exactly one normal per point"
        );
        assert!(
            points.len() == width.len(),
            "there must be exactly one width per point"
        );
        // find the vector of each line segment
        let dposition_per_segment: Vec<Vector3<f32>> = points
            .windows(2)
            .map(|w| w[1] - w[0])
            .collect();

        // dposition_per_points[0] = dposition_per_segment[0] and dposition_per_points[n] = dposition_per_segment[n-1], but it is the average of the two for the points in between
        let dposition_per_points: Vec<Vector3<f32>> = {
            let mut dposition_per_points = Vec::new();
            dposition_per_points.push(dposition_per_segment[0]);
            for i in 1..dposition_per_segment.len() {
                dposition_per_points
                    .push((dposition_per_segment[i - 1] + dposition_per_segment[i]).normalize());
            }
            dposition_per_points.push(dposition_per_segment[dposition_per_segment.len() - 1]);
            dposition_per_points
        };

        // find the cross vectors (along which the width will be applied)
        let cross_vectors: Vec<Vector3<f32>> = dposition_per_points
            .iter()
            .zip(normals.iter())
            .map(|(&v, &n)| v.cross(n).normalize())
            .collect();

        // find the left and right points
        let left_points: Vec<Vector3<f32>> = cross_vectors
            .iter()
            .zip(width.iter())
            .zip(points.iter())
            .map(|((v, &w), p)| p - v * w)
            .collect();

        let right_points: Vec<Vector3<f32>> = cross_vectors
            .iter()
            .zip(width.iter())
            .zip(points.iter())
            .map(|((v, &w), p)| p + v * w)
            .collect();

        let vertexes: Vec<Vertex> = std::iter::zip(left_points.windows(2), right_points.windows(2))
            .flat_map(|(l, r)| {
                vec![
                    Vertex {
                        loc: [l[0][0], l[0][1], l[0][2]],
                        color: [0.9, 0.5, 0.5, 1.0],
                    },
                    Vertex {
                        loc: [l[1][0], l[1][1], l[1][2]],
                        color: [0.5, 0.9, 0.5, 1.0],
                    },
                    Vertex {
                        loc: [r[0][0], r[0][1], r[0][2]],
                        color: [0.5, 0.5, 0.9, 1.0],
                    },
                    Vertex {
                        loc: [r[0][0], r[0][1], r[0][2]],
                        color: [0.9, 0.5, 0.5, 1.0],
                    },
                    Vertex {
                        loc: [l[1][0], l[1][1], l[1][2]],
                        color: [0.5, 0.9, 0.5, 1.0],
                    },
                    Vertex {
                        loc: [r[1][0], r[1][1], r[1][2]],
                        color: [0.5, 0.5, 0.9, 1.0],
                    },
                ]
            })
            .collect();

        Object::new(vertexes)
    }
}
