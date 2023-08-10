use std::sync::Arc;

use nalgebra::Matrix4;

use parking_lot::RwLock;

use super::{
    ObjectAllocator,
	object::ObjectTransform,
	object::{
		Object,
		model::Model,
		texture::Texture
	}
};

use crate::transform::Transform;


pub struct ObjectInfo
{
    pub model: Arc<RwLock<Model>>,
    pub texture: Arc<RwLock<Texture>>,
    pub transform: Transform,
    pub projection_view: Matrix4<f32>
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
            info.projection_view,
			info.model,
			info.texture,
			object_transform,
			&self.allocator
		)
	}
}
