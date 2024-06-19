use std::fmt;

use vulkano::buffer::Subbuffer;

use nalgebra::{Vector3, Vector4, Matrix4};

use crate::{
    game_object::*,
    object::{Model, ObjectVertex, ObjectTransform},
    allocators::ObjectAllocator,
    transform::{Transform, OnTransformCallback, TransformContainer}
};


pub struct OccludingPlane
{
    transform: ObjectTransform,
    subbuffers: Box<[Subbuffer<[ObjectVertex]>]>
}

#[allow(dead_code)]
impl OccludingPlane
{
    pub fn new_default(
        allocator: &ObjectAllocator
    ) -> Self
    {
        let transform = ObjectTransform::new_default();

        Self::new(transform, allocator)
    }

    pub fn new(
        transform: ObjectTransform,
        allocator: &ObjectAllocator
    ) -> Self
    {
        let subbuffers = allocator.subbuffers(&Model::square(1.0));

        Self{
            transform,
            subbuffers
        }
    }

    fn calculate_vertices(
        &self,
        unscaled_origin: Vector3<f32>,
        projection_view: Matrix4<f32>
    ) -> Box<[ObjectVertex]>
    {
        let transform = self.transform.matrix();

        let bottom_left = transform * Vector4::new(-0.5, 0.0, 0.0, 1.0);
        let bottom_right = transform * Vector4::new(0.5, 0.0, 0.0, 1.0);

        let with_w = |values: Vector3<f32>, w|
        {
            Vector4::new(values.x, values.y, values.z, w)
        };

        let origin = unscaled_origin * 2.0;

        let mut top_left = bottom_left.xyz() + bottom_left.xyz() - origin;
        top_left.z = bottom_left.z;

        let mut top_right = bottom_right.xyz() + bottom_right.xyz() - origin;
        top_right.z = bottom_right.z;

        let bottom_left = projection_view * bottom_left;
        let bottom_right = projection_view * bottom_right;
        let top_left = projection_view * with_w(top_left, 0.0);
        let top_right = projection_view * with_w(top_right, 0.0);

        let cross_product = (bottom_right.xyz() - bottom_left.xyz())
            .cross(&(top_left.xyz() - bottom_left.xyz()));

        let winding = {
            let mut bottom_left = bottom_left.xyz();
            bottom_left.z = unscaled_origin.z;

            bottom_left.dot(&cross_product)
        };

        let clockwise = winding > 0.0;

        let vertices = if clockwise
        {
            vec![
                bottom_left,
                top_left,
                bottom_right,
                top_left,
                top_right,
                bottom_right
            ]
        } else
        {
            vec![
                top_right,
                top_left,
                bottom_right,
                top_left,
                bottom_left,
                bottom_right
            ]
        };

        let uvs = Model::square(1.0).uvs;

        vertices.iter().zip(uvs.iter()).map(move |(&vertex, uv)|
        {
            ObjectVertex{position: vertex.into(), uv: *uv}
        }).collect::<Box<[_]>>()
    }

    pub fn update_buffers(
        &mut self,
        origin: Vector3<f32>,
        info: &mut UpdateBuffersInfo
    )
    {
        info.object_info.partial.builder_wrapper.builder()
            .update_buffer(
                self.subbuffers[info.object_info.partial.image_index].clone(),
                self.calculate_vertices(origin, info.object_info.projection_view)
            ).unwrap();
    }

    pub fn draw(&self, info: &mut DrawInfo)
    {
        let square_vertices = Model::square(1.0).vertices.len() as u32;

        info.object_info.builder_wrapper.builder()
            .bind_vertex_buffers(0, self.subbuffers[info.object_info.image_index].clone())
            .unwrap()
            .draw(square_vertices, 1, 0, 0)
            .unwrap();
    }
}

impl OnTransformCallback for OccludingPlane
{
    fn callback(&mut self)
    {
        self.transform.callback();
    }
}

impl TransformContainer for OccludingPlane
{
    fn transform_ref(&self) -> &Transform
    {
        self.transform.transform_ref()
    }

    fn transform_mut(&mut self) -> &mut Transform
    {
        self.transform.transform_mut()
    }
}

impl fmt::Debug for OccludingPlane
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("OccludingPlane")
            .field("transform", &self.transform)
            .finish()
    }
}
