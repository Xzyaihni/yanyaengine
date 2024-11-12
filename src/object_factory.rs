use std::sync::Arc;

use parking_lot::RwLock;

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
    pub texture: Arc<RwLock<Texture>>,
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

    pub fn create_solid(&self, model: Arc<RwLock<Model>>, transform: Transform) -> SolidObject
    {
        SolidObject::new(
            model,
            ObjectTransform::new_transformed(transform),
            &self.allocator
        )
    }

    pub fn create_occluding(&self, transform: Transform) -> OccludingPlane
    {
		let object_transform = ObjectTransform::new_transformed(transform);

        OccludingPlane::new(object_transform, &self.allocator)
    }
}
