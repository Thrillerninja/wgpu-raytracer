use rtbvh::*;
use std::cmp::Ordering;

use crate::structs::{Material, Triangle, Sphere};

fn build_bvh(triangles: &[Triangle]) -> Bvh {
    let mut builder = Builder::default();

    // Recursive function to build the BVH
    fn build_recursive(triangles: &mut [Triangle]) -> Bvh {
        if triangles.len() <= 4 {
            // Create a leaf node if there are few triangles
            let aabb = calculate_aabb(triangles);
            Bvh::Leaf(triangles.to_vec(), aabb)
        } else {
            // Sort the triangles along the largest dimension
            let (split_axis, split_value) = find_split(triangles);

            // Partition triangles into two sets based on the split value
            let (left, right) = triangles.partition(|t| {
                let centroid = calculate_triangle_centroid(t);
                centroid[split_axis] < split_value
            });

            // Recursively build the left and right subtrees
            let left_bvh = build_recursive(left);
            let right_bvh = build_recursive(right);

            // Create a node for the BVH with the left and right subtrees
            Bvh::Node {
                left: Box::new(left_bvh),
                right: Box::new(right_bvh),
                aabb: left_bvh.bounding_box().expand(&right_bvh.bounding_box()),
            }
        }
    }

    // Start the BVH construction with a mutable slice of triangles
    let bvh = build_recursive(&mut triangles.to_vec());

    builder.add_bvh(&bvh);

    builder.build()
}

// Create a function to calculate the AABB for a group of triangles
fn calculate_aabb(triangles: &[Triangle]) -> Aabb {
    let mut aabb = Aabb::empty();

    for triangle in triangles {
        // Calculate the AABB for each triangle and expand the overall AABB
        // You need to implement a function to calculate the AABB for a triangle.
        // This function should return the minimum and maximum points of the triangle.
        let (min, max) = calculate_triangle_aabb(triangle);
        aabb.grow_bb(&Aabb { min: min.into(), extra1: 0, max: max.into(), extra2: 0 });
    }

    aabb
}

fn calculate_triangle_aabb(triangle: &Triangle) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];

    for i in 0..3 {
        for j in 0..3 {
            min[j] = min[j].min(triangle.points[i][j]);
            max[j] = max[j].max(triangle.points[i][j]);
        }
    }

    (min, max)
}

fn calculate_triangle_centroid(triangle: &Triangle) -> [f32; 3] {
    let mut centroid = [0.0; 3];

    for i in 0..3 {
        for j in 0..3 {
            centroid[j] += triangle.points[i][j];
        }
    }

    for j in 0..3 {
        centroid[j] /= 3.0;
    }

    centroid
}

fn find_split(triangles: &[Triangle]) -> (usize, f32) {
    // Calculate the overall AABB for all triangles
    let mut overall_aabb = calculate_aabb(triangles);

    // Find the axis (0, 1, or 2) with the largest extent
    let mut split_axis = 0;
    let mut max_extent = overall_aabb.max[0] - overall_aabb.min[0];

    for axis in 1..3 {
        let extent = overall_aabb.max[axis] - overall_aabb.min[axis];
        if extent > max_extent {
            max_extent = extent;
            split_axis = axis;
        }
    }

    // Sort the triangles along the selected axis
    triangles.sort_by(|a, b| {
        let a_centroid = calculate_triangle_centroid(a)[split_axis];
        let b_centroid = calculate_triangle_centroid(b)[split_axis];
        a_centroid.partial_cmp(&b_centroid).unwrap_or(Ordering::Equal)
    });

    // Find the best split value by minimizing the overlap of AABBs
    let mut split_index = 0;
    let mut split_value = calculate_triangle_centroid(&triangles[0])[split_axis];

    let mut min_right: f32 = calculate_aabb(&triangles[0..1]).grow_bb(&calculate_aabb(&triangles[1..]));

    for (i, triangle) in triangles.iter().enumerate().skip(1) {
        let triangle_centroid = calculate_triangle_centroid(triangle)[split_axis];
        let left_aabb = calculate_aabb(&triangles[0..=i]);
        let right_aabb = calculate_aabb(&triangles[(i + 1)..]);

        let overlap = left_aabb.grow_bb(&right_aabb);
        let right_extend = right_aabb.max[split_axis] - right_aabb.min[split_axis];

        if overlap < min_right && right_extend < min_right {
            min_right = overlap;
            split_index = i;
            split_value = triangle_centroid;
        }
    }

    (split_axis, split_value)
}
