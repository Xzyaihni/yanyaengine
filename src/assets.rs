use std::{
    fs,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc
};

use parking_lot::RwLock;

use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::{
    PipelineInfo,
    object::{
        resource_uploader::ResourceUploader,
        model::Model,
        texture::{RgbaImage, Texture}
    }
};


#[derive(EnumIter)]
pub enum DefaultModel
{
    Square
}

pub struct Assets
{
	texture_ids: HashMap<String, usize>,
	textures: Vec<Arc<RwLock<Texture>>>,
	model_ids: HashMap<String, usize>,
	models: Vec<Arc<RwLock<Model>>>,
    default_models: Vec<Arc<RwLock<Model>>>
}

impl Assets
{
    pub fn new<TexturesPath, ModelsPath>(
        resource_uploader: &mut ResourceUploader,
        textures_path: Option<TexturesPath>,
        models_path: Option<ModelsPath>
    ) -> Self
    where
        TexturesPath: AsRef<Path>,
        ModelsPath: AsRef<Path>
    {
        let (textures, texture_ids) = Self::load_named_assets(textures_path, |path|
        {
			let image = RgbaImage::load(path).unwrap();

            Texture::new(resource_uploader, image)
        });

        let (models, model_ids) = Self::load_named_assets(models_path, |path|
        {
            Model::load(path).unwrap()
        });

        let default_models = Self::create_default_models();

        Self{texture_ids, textures, model_ids, models, default_models}
    }

    pub fn default_model(&self, id: DefaultModel) -> Arc<RwLock<Model>>
    {
        self.default_models[id as usize].clone()
    }

    pub fn texture(&self, name: &str) -> Arc<RwLock<Texture>>
    {
        self.textures[self.texture_ids[name]].clone()
    }

    pub fn model(&self, name: &str) -> Arc<RwLock<Model>>
    {
        self.models[self.model_ids[name]].clone()
    }

    pub fn add_textures<T>(&mut self, textures: T)
    where
        T: IntoIterator<Item=(String, Texture)>
    {
        Self::add_assets(textures.into_iter(), &mut self.textures, &mut self.texture_ids);
    }

    pub fn add_models<T>(&mut self, models: T)
    where
        T: IntoIterator<Item=(String, Model)>
    {
        Self::add_assets(models.into_iter(), &mut self.models, &mut self.model_ids);
    }

    fn create_default_models() -> Vec<Arc<RwLock<Model>>>
    {
        DefaultModel::iter().map(|default_model|
        {
            let model;

            match default_model
            {
                DefaultModel::Square =>
                {
                    model = Model::square(1.0);
                }
            }

            Arc::new(RwLock::new(model))
        }).collect()
    }

    fn add_assets<AddAssetsType, T>(
        add_assets: AddAssetsType,
        assets: &mut Vec<Arc<RwLock<T>>>,
        asset_ids: &mut HashMap<String, usize>
    )
    where
        AddAssetsType: Iterator<Item=(String, T)>
    {
        let index_offset = assets.len();
        let (insert_assets, map_assets): (Vec<_>, Vec<(_, usize)>) =
            add_assets.enumerate().map(|(index, (name, asset))|
            {
                (Arc::new(RwLock::new(asset)), (name, index_offset + index))
            }).unzip();

        assets.extend(insert_assets);
        asset_ids.extend(map_assets);
    }

	pub fn swap_pipeline(&mut self, info: &PipelineInfo)
	{
		self.textures.iter_mut().for_each(|texture|
		{
			texture.write().swap_pipeline(info)
		});
	}

    fn load_named_assets<P, F, T>(
        folder_path: Option<P>,
        mut loader: F
    ) -> (Vec<Arc<RwLock<T>>>, HashMap<String, usize>)
    where
        P: AsRef<Path>,
        F: FnMut(PathBuf) -> T
    {
        if folder_path.is_none()
        {
            let assets = Vec::new();
            let asset_indices = HashMap::new();

            return (assets, asset_indices);
        }

        let folder_path = folder_path.unwrap();

		let assets = Self::recursive_dir(folder_path.as_ref()).into_iter().map(|name|
		{
            let value = Arc::new(RwLock::new(loader(name.clone())));

			let short_path = name.iter().skip(1).fold(PathBuf::new(), |mut acc, part|
			{
				acc.push(part);

				acc
			}).into_os_string().into_string().unwrap();

			(short_path, value)
		}).collect::<HashMap<_, _>>();

        let (indices_map, data) = assets.into_iter().enumerate().map(|(index, (name, asset))|
        {
            let named_index = (name, index);
            let value = asset;

            (named_index, value)
        }).unzip();

        (data, indices_map)
    }

	fn recursive_dir(path: &Path) -> impl Iterator<Item=PathBuf>
	{
		let mut collector = Vec::new();

		Self::recursive_dir_inner(path, &mut collector);

		collector.into_iter()
	}

	fn recursive_dir_inner(path: &Path, collector: &mut Vec<PathBuf>)
	{
		fs::read_dir(path).unwrap().flatten().for_each(|entry|
		{
			let path = entry.path();
			if path.is_dir()
			{
				Self::recursive_dir_inner(&path, collector);
			} else
			{
				collector.push(entry.path());
			}
		})
	}
}
