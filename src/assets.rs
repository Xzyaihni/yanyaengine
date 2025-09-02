use std::{
    fs,
    fmt,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    ops::{Index, IndexMut}
};

use parking_lot::{RwLock, Mutex};

use strum::{IntoEnumIterator, EnumIter, IntoStaticStr};

use serde::{Serialize, Deserialize};

use crate::{
    UpdateBuffersInfo,
    BuilderWrapper,
    object::{
        resource_uploader::ResourceUploader,
        model::Model,
        texture::{Color, SimpleImage, RgbaImage, Texture}
    }
};


#[derive(EnumIter, IntoStaticStr)]
pub enum DefaultModel
{
    Square
}

#[derive(EnumIter, IntoStaticStr)]
pub enum DefaultTexture
{
    Solid
}

pub struct NamedValue<T>
{
    pub name: String,
    pub value: T
}

impl<T> NamedValue<T>
{
    pub fn map<U, F>(self, f: F) -> NamedValue<U>
    where
        F: FnOnce(T) -> U
    {
        let NamedValue{
            name,
            value
        } = self;

        NamedValue{
            name,
            value: f(value)
        }
    }
}

pub struct FilesLoader;

impl FilesLoader
{
    pub fn load_images(folder_path: impl AsRef<Path>) -> impl Iterator<Item=NamedValue<RgbaImage>>
    {
        Self::load(folder_path).filter_map(|named_value|
        {
            let image = match RgbaImage::load(named_value.value)
            {
                Ok(x) => x,
                Err(err) =>
                {
                    eprintln!("error loading {}: {err}", &named_value.name);
                    return None;
                }
            };

            Some(NamedValue{
                name: named_value.name,
                value: image
            })
        })
    }

    pub fn load(folder_path: impl AsRef<Path>) -> impl Iterator<Item=NamedValue<PathBuf>>
    {
        Self::recursive_dir(folder_path.as_ref()).map(move |name|
        {
            let value = name.clone();

            let short_path = name.strip_prefix(folder_path.as_ref())
                .expect("all paths must be in parent folder")
                .to_string_lossy().into_owned();

            NamedValue{name: short_path, value}
        })
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

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize, bincode::Decode, bincode::Encode)]
pub struct TextureId(usize);

impl From<usize> for TextureId
{
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<TextureId> for usize
{
    fn from(value: TextureId) -> usize
    {
        value.0
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize, bincode::Decode, bincode::Encode)]
pub struct ModelId(usize);

impl From<usize> for ModelId
{
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<ModelId> for usize
{
    fn from(value: ModelId) -> usize
    {
        value.0
    }
}

struct IdsStorage<I, T>
{
    ids: HashMap<String, I>,
    data: Vec<T>
}

impl<I, T> FromIterator<(String, T)> for IdsStorage<I, T>
where
    I: From<usize> + Clone
{
    fn from_iter<Iter>(iter: Iter) -> Self
    where
        Iter: IntoIterator<Item=(String, T)>
    {
        let mut this = Self::default();

        this.extend(iter);

        this
    }
}

impl<I, T> Extend<(String, T)> for IdsStorage<I, T>
where
    I: From<usize> + Clone
{
    fn extend<Iter>(&mut self, iter: Iter)
    where
        Iter: IntoIterator<Item=(String, T)>
    {
        iter.into_iter().for_each(|item| { self.insert(item); });
    }
}

impl<I, T> Default for IdsStorage<I, T>
{
    fn default() -> Self
    {
        Self{ids: HashMap::new(), data: Vec::new()}
    }
}

impl<I, T> IdsStorage<I, T>
{
    pub fn insert(&mut self, item: (String, T)) -> I
    where
        I: From<usize> + Clone
    {
        let id: I = self.data.len().into();

        self.ids.insert(item.0.replace('\\', "/"), id.clone());
        self.data.push(item.1);

        id
    }

    pub fn push(&mut self, item: T) -> I
    where
        I: From<usize>
    {
        let id: I = self.data.len().into();

        self.data.push(item);

        id
    }

    pub fn get_id(&self, name: &str) -> Option<&I>
    {
        self.ids.get(name)
    }

    pub fn keys(&self) -> impl Iterator<Item=&String>
    {
        self.ids.keys()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut T>
    {
        self.data.iter_mut()
    }
}

impl<I, T> Index<I> for IdsStorage<I, T>
where
    I: Into<usize>
{
    type Output = T;

    fn index(&self, index: I) -> &Self::Output
    {
        &self.data[index.into()]
    }
}

impl<I, T> IndexMut<I> for IdsStorage<I, T>
where
    I: Into<usize>
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output
    {
        &mut self.data[index.into()]
    }
}

pub struct Assets
{
    textures_path: Option<PathBuf>,
    models_path: Option<PathBuf>,
    textures: IdsStorage<TextureId, Arc<Mutex<Texture>>>,
    models: IdsStorage<ModelId, Arc<RwLock<Model>>>
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
        let output_textures_path = textures_path.as_ref().map(|x| x.as_ref().to_owned());
        let output_models_path = models_path.as_ref().map(|x| x.as_ref().to_owned());

        let mut textures = Self::load_resource(textures_path, |path|
        {
            FilesLoader::load_images(path).map(|named_value|
            {
                named_value.map(|image|
                {
                    Texture::new(resource_uploader, image)
                })
            })
        }, |x| Arc::new(Mutex::new(x)));

        textures.extend(Self::create_default_textures(resource_uploader));

        let mut models = Self::load_resource(models_path, |path|
        {
            FilesLoader::load(path).map(|named_value|
            {
                named_value.map(|path| Model::load(path).unwrap())
            })
        }, |x| Arc::new(RwLock::new(x)));

        models.extend(Self::create_default_models());

        Self{
            textures_path: output_textures_path,
            models_path: output_models_path,
            textures,
            models
        }
    }

    pub fn reload(&mut self, info: &mut UpdateBuffersInfo)
    {
        let textures_path = self.textures_path.clone();
        let models_path = self.models_path.clone();

        *self = Self::new(info.partial.builder_wrapper.resource_uploader_mut(), textures_path, models_path);
    }

    fn load_resource<Id, T, U, F, I, P>(
        maybe_path: Option<P>,
        f: F,
        m: impl Fn(T) -> U
    ) -> IdsStorage<Id, U>
    where
        Id: From<usize> + Clone,
        P: AsRef<Path>,
        I: Iterator<Item=NamedValue<T>>,
        F: FnOnce(P) -> I
    {
        maybe_path.map(|path|
        {
            f(path).map(|NamedValue{name, value}|
            {
                (name, m(value))
            }).collect()
        }).unwrap_or_default()
    }

    pub fn default_model(&self, id: DefaultModel) -> ModelId
    {
        self.try_model_id(id.into()).unwrap()
    }

    pub fn default_texture(&self, id: DefaultTexture) -> TextureId
    {
        self.try_texture_id(id.into()).unwrap()
    }

    pub fn try_texture_id(&self, name: &str) -> Option<TextureId>
    {
        self.textures.get_id(name).copied()
    }

    pub fn texture_id(&self, name: &str) -> TextureId
    {
        self.try_texture_id(name).unwrap_or_else(||
        {
            eprintln!("texture named `{name}` doesnt exist, using fallback");

            self.default_texture(DefaultTexture::Solid)
        })
    }

    pub fn try_texture_by_name<'a>(&'a self, name: &str) -> Option<&'a Arc<Mutex<Texture>>>
    {
        Some(&self.textures[self.try_texture_id(name)?])
    }

    pub fn texture_by_name<'a>(&'a self, name: &str) -> &'a Arc<Mutex<Texture>>
    {
        &self.textures[self.texture_id(name)]
    }

    pub fn texture(&self, id: TextureId) -> &Arc<Mutex<Texture>>
    {
        &self.textures[id]
    }

    pub fn try_model_id(&self, name: &str) -> Option<ModelId>
    {
        self.models.get_id(name).copied()
    }

    pub fn model_id(&self, name: &str) -> ModelId
    {
        self.try_model_id(name).unwrap_or_else(||
        {
            eprintln!("model named `{name}` doesnt exist, using fallback");

            self.default_model(DefaultModel::Square)
        })
    }

    pub fn try_model_by_name<'a>(&'a self, name: &str) -> Option<&'a Arc<RwLock<Model>>>
    {
        Some(&self.models[self.try_model_id(name)?])
    }

    pub fn model_by_name<'a>(&'a self, name: &str) -> &'a Arc<RwLock<Model>>
    {
        &self.models[self.model_id(name)]
    }

    pub fn model(&self, id: ModelId) -> &Arc<RwLock<Model>>
    {
        &self.models[id]
    }

    pub fn edited_copy(
        &mut self,
        builder_wrapper: &mut BuilderWrapper,
        name: &str,
        f: impl FnOnce(&mut SimpleImage)
    ) -> TextureId
    {
        let textures_path = self.textures_path.as_ref().expect("cant edit empty assets");
        let filepath = textures_path.join(name);

        let mut image = SimpleImage::load(&filepath).unwrap();
        f(&mut image);

        let texture = builder_wrapper.create_texture(image.into());

        self.textures.insert((name.to_owned(), Arc::new(Mutex::new(texture))))
    }

    pub fn add_textures<T>(&mut self, textures: T)
    where
        T: IntoIterator<Item=(String, Texture)>
    {
        self.textures.extend(textures.into_iter().map(|(a, b)| (a, Arc::new(Mutex::new(b)))))
    }

    pub fn add_models<T>(&mut self, models: T)
    where
        T: IntoIterator<Item=(String, Model)>
    {
        self.models.extend(models.into_iter().map(|(a, b)| (a, Arc::new(RwLock::new(b)))));
    }

    pub fn push_texture(&mut self, texture: Texture) -> TextureId
    {
        self.textures.push(Arc::new(Mutex::new(texture)))
    }

    pub fn push_model(&mut self, model: Model) -> ModelId
    {
        self.models.push(Arc::new(RwLock::new(model)))
    }

    fn create_default_textures<'a, 'b>(
        resource_uploader: &'a mut ResourceUploader<'b>
    ) -> impl Iterator<Item=(String, Arc<Mutex<Texture>>)> + use<'a, 'b>
    {
        DefaultTexture::iter().map(|default_texture|
        {
            let texture = match default_texture
            {
                DefaultTexture::Solid =>
                {
                    Texture::new(
                        resource_uploader,
                        SimpleImage::filled(Color{r: 255, g: 255, b: 255, a: 255}, 1, 1).into()
                    )
                }
            };

            let name: &str = default_texture.into();
            (name.to_owned(), Arc::new(Mutex::new(texture)))
        })
    }

    fn create_default_models() -> impl Iterator<Item=(String, Arc<RwLock<Model>>)>
    {
        DefaultModel::iter().map(|default_model|
        {
            let model = match default_model
            {
                DefaultModel::Square =>
                {
                    Model::square(1.0)
                }
            };

            let name: &str = default_model.into();
            (name.to_owned(), Arc::new(RwLock::new(model)))
        })
    }

    pub fn swap_pipelines(&mut self)
    {
        self.textures.iter_mut().for_each(|texture|
        {
            texture.lock().swap_pipeline()
        });
    }
}

impl fmt::Debug for Assets
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        let texture_names = self.textures.keys().map(|x| x.to_owned())
            .reduce(|acc, v|
            {
                acc + ", " + &v
            }).unwrap_or_default();

        let model_names = self.models.keys().map(|x| x.to_owned())
            .reduce(|acc, v|
            {
                acc + ", " + &v
            }).unwrap_or_default();

        write!(f, "Assets {{ textures: [{}], models: [{}] }}", texture_names, model_names)
    }
}
