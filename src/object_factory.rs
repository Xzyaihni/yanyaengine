use std::{fmt, sync::Arc};

use parking_lot::{RwLock, Mutex};

use vulkano::pipeline::graphics::vertex_input::Vertex;

use super::{
    OccludingPlane,
    allocators::ObjectAllocator,
	object::ObjectTransform,
	object::{
		Object,
		model::Model,
		texture::Texture
	}
};

use crate::{
    SolidObject,
    transform::Transform
};


pub struct ObjectInfo
{
    pub model: Arc<RwLock<Model>>,
    pub texture: Arc<Mutex<Texture>>,
    pub transform: Transform
}

#[derive(Debug)]
pub struct ObjectFactory
{
	allocator: ObjectAllocator
}

impl ObjectFactory
{
	pub fn new(allocator: ObjectAllocator) -> Self
	{
		Self{allocator}
	}

	pub fn create(&self, info: ObjectInfo) -> Object
	{
		let object_transform = ObjectTransform::new_transformed(info.transform);

		Object::new(
			info.model,
			info.texture,
			object_transform,
			&self.allocator
		)
	}

    pub fn create_solid<VertexType: Vertex + From<([f32; 4], [f32; 2])>>(
        &self,
        model: Arc<RwLock<Model>>,
        transform: Transform
    ) -> SolidObject<VertexType>
    {
        SolidObject::new(
            model,
            ObjectTransform::new_transformed(transform),
            &self.allocator
        )
    }

    pub fn create_occluding<VertexType: Vertex + From<[f32; 4]> + fmt::Debug>(
        &self,
        transform: Transform,
        reverse_winding: bool
    ) -> OccludingPlane<VertexType>
    {
		let object_transform = ObjectTransform::new_transformed(transform);

        OccludingPlane::new(object_transform, reverse_winding, &self.allocator)
    }
}
