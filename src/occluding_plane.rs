#[allow(unused_imports)]
use std::{fmt, cell::RefCell};

use vulkano::{
    buffer::Subbuffer,
    pipeline::{PipelineBindPoint, graphics::vertex_input::{VertexBufferDescription, Vertex}}
};

use nalgebra::{Vector3, Vector4, Matrix4};

use crate::{
    WINDING_MATTERS,
    game_object::*,
    SimpleVertex,
    object::{impl_updated_check, Model, ObjectTransform},
    allocators::ObjectAllocator,
    transform::{Transform, OnTransformCallback, TransformContainer}
};


pub struct OccludingPlane
{
    transform: ObjectTransform,
    subbuffer: Subbuffer<[SimpleVertex]>,
    #[cfg(debug_assertions)]
    updated_buffers: Option<bool>
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
        let subbuffer = allocator.subbuffer(Model::square(1.0).vertices.len() as u64);

        Self{
            transform,
            subbuffer,
            #[cfg(debug_assertions)]
            updated_buffers: None
        }
    }

    fn calculate_vertices(
        &self,
        origin: Vector3<f32>,
        projection_view: Matrix4<f32>
    ) -> Box<[SimpleVertex]>
    {
        let transform = self.transform.matrix();

        let un_bottom_left = transform * Vector4::new(-0.5, 0.0, 0.0, 1.0);
        let un_bottom_right = transform * Vector4::new(0.5, 0.0, 0.0, 1.0);

        let with_w = |values: Vector3<f32>, w|
        {
            Vector4::new(values.x, values.y, values.z, w)
        };

        let mut un_top_left = un_bottom_left.xyz() - origin;
        un_top_left.z = 0.0;

        let mut un_top_right = un_bottom_right.xyz() - origin;
        un_top_right.z = 0.0;

        let bottom_left = projection_view * un_bottom_left;
        let mut bottom_right = projection_view * un_bottom_right;
        let mut top_left = projection_view * with_w(un_top_left, 0.0);
        let mut top_right = projection_view * with_w(un_top_right, 0.0);

        {
            let z = bottom_left.z;

            bottom_right.z = z;
            top_left.z = z;
            top_right.z = z;
        }

        let winding = {
            let un_top_left = un_bottom_left.xyz() + un_bottom_left.xyz() - origin;
            let top_left = (projection_view * with_w(un_top_left, 1.0)).xy();

            let bottom_left = bottom_left.xy();
            let bottom_right = bottom_right.xy();

            let i0 = bottom_right - bottom_left;
            let i1 = top_left - bottom_left;

            i0.x * i1.y - i0.y * i1.x
        };

        let clockwise = WINDING_MATTERS && winding > 0.0;

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

        vertices.iter().map(move |&vertex|
        {
            SimpleVertex{position: vertex.into()}
        }).collect::<Box<[_]>>()
    }

    pub fn update_buffers(
        &mut self,
        origin: Vector3<f32>,
        info: &mut UpdateBuffersInfo
    )
    {
        self.set_updated(&info.partial);

        info.partial.builder_wrapper.builder()
            .update_buffer(
                self.subbuffer.clone(),
                self.calculate_vertices(origin, info.projection_view)
            ).unwrap();
    }

    pub fn draw(&self, info: &mut DrawInfo)
    {
        self.assert_updated(&info.object_info);

        let square_vertices = Model::square(1.0).vertices.len() as u32;

        let layout = info.current_layout();

        unsafe{
            info.object_info.builder_wrapper.builder()
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    layout,
                    0,
                    info.current_sets.clone()
                )
                .unwrap()
                .bind_vertex_buffers(0, self.subbuffer.clone())
                .unwrap()
                .draw(square_vertices, 1, 0, 0)
                .unwrap();
        }
    }

    impl_updated_check!{}

    pub fn per_vertex() -> VertexBufferDescription
    {
        SimpleVertex::per_vertex()
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
