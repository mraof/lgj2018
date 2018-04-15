#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate image;
extern crate indexmap;
extern crate tiled;
extern crate rlua;
extern crate palette;
extern crate ncollide;
extern crate nalgebra;

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::time::Instant;
use std::sync::Arc;
use indexmap::IndexMap;
use gfx::traits::{Factory, FactoryExt};
use gfx::handle::ShaderResourceView;
use glutin::GlContext;
use rlua::{Lua, UserData, UserDataMethods, MetaMethod};
use tiled::Tileset;
use nalgebra::{Vector2, Isometry2};
use ncollide::{events::ContactEvent, shape::{ShapeHandle2, Cuboid2, Plane2}, world::{CollisionWorld2, CollisionGroups, GeometricQueryType, CollisionObjectHandle}};

pub type ColorFormat = gfx::format::Srgba8;
pub type DepthFormat = gfx::format::DepthStencil;
pub type TextureColorFormat = (gfx::format::R8, gfx::format::Uint);

const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

gfx_defines! {
    vertex Vertex {
        pos: [f32; 2] = "pos",
        uv: [f32; 2] = "uv",
    }
    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        palette: gfx::TextureSampler<[f32; 4]> = "palette",
        sprite: gfx::TextureSampler<u32> = "sprite",
        x: gfx::Global<i32> = "x",
        y: gfx::Global<i32> = "y",
        width: gfx::Global<f32> = "width",
        height: gfx::Global<f32> = "height",
        flip: gfx::Global<f32> = "flip",
        out: gfx::BlendTarget<ColorFormat> = ("Target0", gfx::state::ColorMask::all(), gfx::preset::blend::ALPHA),
    }
}

fn main() {
    let instant = Instant::now();
    let mut events_loop = glutin::EventsLoop::new();
    let context = glutin::ContextBuilder::new()
        .with_vsync(true);
    let builder = glutin::WindowBuilder::new()
        .with_title("Umbrella is a verb")
        .with_dimensions(768, 768);

    let width = 2048;
    let height = 2048;

    let (window, mut device, mut factory, main_color, mut main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(builder, context, &events_loop);

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let pso = factory.create_pipeline_simple(
        include_bytes!("shaders/shader.vert"),
        include_bytes!("shaders/shader.frag"),
        pipe::new(),
    ).unwrap();

    let slice = gfx::Slice {
        start: 0,
        end: 8,
        base_vertex: 0,
        instances: None,
        buffer: factory.create_index_buffer(INDICES),
    };

    let palette_sampler = factory.create_sampler(gfx::texture::SamplerInfo::new(gfx::texture::FilterMethod::Scale, gfx::texture::WrapMode::Clamp));
    let sampler = factory.create_sampler(gfx::texture::SamplerInfo::new(gfx::texture::FilterMethod::Scale, gfx::texture::WrapMode::Tile));

    let mut graphics = Graphics::new(factory);

    let lua = Lua::new();

    let mut tiles = Tiles::new(&lua);
    let mut map = Map::load(&mut graphics, &mut tiles, "assets/tiled/Finite.tmx");

    let mut data = {
        let tile = &tiles.tiles[0];
        let texture = graphics.get_texture(tile.texture);

        pipe::Data {
            vbuf: texture.vertex_buffers[0].clone(),
            palette: (graphics.get_palette(texture.palette).get(0), palette_sampler),
            sprite: (texture.texture.clone(), sampler),
            x: 0,
            y: 0,
            width: width as f32,
            height: height as f32,
            flip: 1.0,
            out: main_color,
        }
    };

    let elapsed = instant.elapsed();
    println!("Loaded in {}", elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1.0e9);

    let mut controls = Controls::default();

    let globals = lua.globals();
    globals.set("controls", controls).unwrap();
    globals.set("gravity", 500.0).unwrap();

    let mut running = true;
    let mut instant = Instant::now();
    let mut counter = 0.0;
    
    while running {
        events_loop.poll_events(|event| {
            use glutin::Event::WindowEvent;
            use glutin::WindowEvent::*;
            match event {
                WindowEvent { event, .. } => {
                    match event {
                        Closed => running = false,
                        Resized(_, _) => {
                            gfx_window_glutin::update_views(&window, &mut data.out, &mut main_depth)
                        }
                        KeyboardInput { device_id: _, input } => {
                            use glutin::{ElementState, VirtualKeyCode::*};
                            let pressed = input.state == ElementState::Pressed;
                            match input.virtual_keycode {
                                Some(Z) => controls.a = pressed,
                                Some(X) => controls.b = pressed,
                                Some(Left) => controls.left = pressed,
                                Some(Right) => controls.right = pressed,
                                Some(Up) => controls.up = pressed,
                                Some(Down) => controls.down = pressed,
                                _ => ()
                            }
                            globals.set("controls", controls).unwrap();
                        }
                        Focused(false) => {
                            controls = Controls::default();
                            globals.set("controls", controls).unwrap();
                        }
                        _ => ()
                    }
                }
                _ => ()
            }
        });
        let elapsed = instant.elapsed();
        instant = Instant::now();
        let delta = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1.0e9;
        counter += delta;
        globals.set("delta", delta).expect("Unable to set delta");
        encoder.clear(&data.out, map.color);
        for ((x, y), map_tile) in &map.map {
            let mut tile = tiles.get(map_tile.tile);
            
            if !tile.animation.is_empty() {
                let frame = (counter / (tile.animation[0].duration as f64 / 1000.0)) as usize % tile.animation.len();
                tile = &tiles.tiles[tile.animation[frame].tile];
            }
            
            let texture = graphics.get_texture(tile.texture);
            data.sprite.0 = texture.texture.clone();
            data.x = x * 256;
            data.y = y * 256;
            data.flip = if map_tile.flipped {-1.0} else {1.0};
            data.palette.0 = graphics.get_palette(texture.palette).get(0);
            data.vbuf = texture.vertex_buffers[map_tile.rotation].clone();
            encoder.draw(&slice, &pso, &data);
        }
        for (handle, &mut (tile, ref mut object)) in &mut map.objects {
            let mut tile = tiles.get(tile);
            globals.set("object", object.clone()).expect("Unable to set object");
            if let Some(ref script) = tile.script {
                script.call::<(), ()>(()).expect("Script errored");
                lua.eval::<()>("update()", Some("update")).expect("Update failed");
                *object = globals.get("object").expect("Object vanished!");
            }
            
            {
                if object.move_x != 0.0 || object.move_y != 0.0 {
                    let (object_shape, object_position) = {
                        let collision_object = map.world.collision_object(*handle).unwrap();
                        (collision_object.shape().clone(), collision_object.position().clone())
                    };
                    map.world.set_position(*handle, Isometry2::new(Vector2::new(object.x + object.move_x, object.y + object.move_y), nalgebra::zero()));
                    map.world.update();
                    let mut time = 1.0;
                    for contact_event in map.world.contact_events() {
                        let collided = match contact_event {
                            ContactEvent::Started(a, b) if a == handle => Some(b),
                            ContactEvent::Started(a, b) if b == handle => Some(a),
                            _ => None
                        };
                        if let Some(collided) = collided {
                            let other_collider = map.world.collision_object(*collided).unwrap();
                            let new_time = ncollide::query::time_of_impact(
                                &object_position, &Vector2::new(object.move_x, object.move_y), object_shape.as_ref(),
                                other_collider.position(), &Vector2::new(0.0, 0.0), other_collider.shape().as_ref(),
                            );
                            match new_time {
                                Some(new_time) if new_time < time => time = new_time,
                                _ => ()
                            }
                            println!("Collided with {} {:?}", collided.0, new_time);
                            println!("{:?}\n{:?}", object_position, other_collider.position());
                            
                        }
                    }
                    object.x += object.move_x * time;
                    object.y += object.move_y * time;
                    object.move_x = 0.0;
                    object.move_y = 0.0;
                    map.world.set_position(*handle, Isometry2::new(Vector2::new(object.x, object.y), nalgebra::zero()));
                    map.world.update();
                }
            }

            if !tile.animation.is_empty() {
                let frame = (counter / (tile.animation[0].duration as f64 / 1000.0)) as usize % tile.animation.len();
                tile = &tiles.tiles[tile.animation[frame].tile];
            }

            let texture = graphics.get_texture(tile.texture);
            data.sprite.0 = texture.texture.clone();
            data.x = object.x as i32;
            data.y = object.y as i32;
            data.flip = if object.flipped {-1.0} else {1.0};
            data.palette.0 = graphics.get_palette(texture.palette).get(0);
            data.vbuf = texture.vertex_buffers[object.rotation].clone();
            encoder.draw(&slice, &pso, &data);
        }
        window.swap_buffers().unwrap();
        encoder.flush(&mut device);
    }
}

#[derive(Clone)]
struct Object {
    x: f64,
    y: f64,
    ///x to attempt to move in the movement step
    move_x: f64,
    ///y to attempt to move in the movement step
    move_y: f64,
    width: f64,
    height: f64,
    rotation: usize,
    flipped: bool,
    key: Arc<rlua::RegistryKey>,
}

impl Object {
    fn new(lua: &Lua, tile: &Tile, x: f64, y: f64, rotation: usize, flipped: bool) -> Object {
        let key = Arc::new(lua.create_registry_value(lua.create_table().unwrap()).unwrap());
        let object = Object {
            x,
            y,
            move_x: 0.0,
            move_y: 0.0,
            width: tile.width as f64,
            height: tile.height as f64,
            rotation,
            flipped,
            key,
        };
        if let Some(ref script) = tile.script {
            lua.globals().set("object", object).expect("Failed to set object global");
            script.call::<(), ()>(()).expect("Script failed");
            lua.eval::<()>("init()", Some("init")).expect("Init failed");
            lua.globals().get("object").expect("Failed to retrieve object")
        } else {
            object
        }
    }
}

impl UserData for Object {
    fn add_methods(methods: &mut UserDataMethods<Self>) {
        use rlua::Value;
        //TODO Make this relative
        methods.add_method_mut("move", |_, object, (x, y): (f64, f64)| {
            let (x, y) = match object.rotation {
                0 => (x, y),
                1 => (-y, x),
                2 => (-x, -y),
                _ => (y, -x)
            };
            object.move_x += x;
            object.move_y += y;
            Ok(())
        });
        methods.add_method_mut("rotate", |_, object, rotation: i64| {
            object.rotation = ((rotation % 4) + 4) as usize % 4;
            Ok(())
        });
        methods.add_method_mut("flip", |_, object, flipped: bool| {
            object.flipped = flipped;
            Ok(())
        });
        methods.add_meta_method(MetaMethod::ToString, |_, object, ()| {
            Ok(format!("x: {}\ny: {}, rotation: {}", object.x, object.y, object.rotation))
        });
        methods.add_meta_method(MetaMethod::Index, |lua: &Lua, object, index: String| {
            Ok(match index.as_str() {
                "x" => {
                    let x = match object.rotation {
                        0 => object.x,
                        1 => object.y,
                        2 => 2048.0 - object.x,
                        _ => 2048.0 - object.y
                    };
                    Value::Number(x)
                }
                "y" => {
                    let y = match object.rotation {
                        0 => object.y + object.height,
                        1 => 2048.0 - object.x,
                        2 => 2048.0 - object.y,
                        _ => object.x + object.height
                    };
                    Value::Number(y)
                }
                "width" => Value::Number(object.width),
                "height" => Value::Number(object.height),
                "rotation" => Value::Integer(object.rotation as i64),
                "flipped" => Value::Boolean(object.flipped),
                index => {
                    lua.registry_value::<rlua::Table>(&object.key).unwrap().get(index).unwrap_or(Value::Nil)
                }
            })
        });
        methods.add_meta_method(MetaMethod::NewIndex, |lua: &Lua, object: &Object, (index, value): (String, Value)| {
            lua.registry_value::<rlua::Table>(&object.key).unwrap().set(index, value).expect("Failed to set registry value");
            Ok(())
        });
    }
}

#[derive(Default, Debug, Copy, Clone)]
struct Controls {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub a: bool,
    pub b: bool,
}

impl UserData for Controls {
    fn add_methods(methods: &mut UserDataMethods<Self>) {
        methods.add_meta_method(MetaMethod::ToString, |_, controls, ()| {
            Ok(format!("{:#?}", controls))
        });
        methods.add_meta_method(MetaMethod::Index, |_, controls, index: String| {
            Ok(match index.as_str() {
                "up" => controls.up,
                "down" => controls.down,
                "left" => controls.left,
                "right" => controls.right,
                "a" => controls.a,
                "b" => controls.b,
                _ => {
                    println!("Unknown index: {}", index);
                    false
                }
            })
        })
    }
}

struct World {
    map: Arc<Map>
}

impl UserData for World {
}

struct MapTile {
    tile: usize,
    rotation: usize,
    flipped: bool,
}

struct Map {
    map: HashMap<(i32, i32), MapTile>,
    objects: HashMap<CollisionObjectHandle, (usize, Object)>,
    color: [f32; 4],
    world: CollisionWorld2<f64, ()>
}

impl Map {
    fn load<R: gfx::Resources, F: gfx::Factory<R>>(graphics: &mut Graphics<R, F>, tiles: &mut Tiles, filename: &str) -> Map {
        let tiled_map = tiled::parse_file(std::path::Path::new(filename)).unwrap();
        let mut map = HashMap::new();
        println!("{:#?}", tiled_map);
        let mut tile_lookup = HashMap::new();
        for tileset in tiled_map.tilesets {
            tile_lookup.extend(tiles.load(graphics, tileset))
        }
        
        let mut world = CollisionWorld2::new(0.02);
        let mut map_groups = CollisionGroups::new();
        map_groups.set_membership(&[1]);
        map_groups.set_whitelist(&[2]);
        let mut object_groups = CollisionGroups::new();
        object_groups.set_membership(&[2]);
        let contacts_query = GeometricQueryType::Contacts(0.0, 0.0);
        world.add(Isometry2::new(Vector2::new(0.0, 0.0), nalgebra::zero()), ShapeHandle2::new(Plane2::new(Vector2::x_axis())), map_groups, contacts_query, ());
        world.add(Isometry2::new(Vector2::new(tiled_map.width as f64 * tiled_map.tile_width as f64, 0.0), nalgebra::zero()), ShapeHandle2::new(Plane2::new(-Vector2::x_axis())), map_groups, contacts_query, ());
        world.add(Isometry2::new(Vector2::new(0.0, 0.0), nalgebra::zero()), ShapeHandle2::new(Plane2::new(Vector2::y_axis())), map_groups, contacts_query, ());
        world.add(Isometry2::new(Vector2::new(0.0, tiled_map.height as f64 * tiled_map.tile_height as f64), nalgebra::zero()), ShapeHandle2::new(Plane2::new(-Vector2::y_axis())), map_groups, contacts_query, ());
        
        for layer in tiled_map.layers {
            for (y, row) in layer.tiles.into_iter().enumerate() {
                for (x, tile) in row.into_iter().enumerate() {
                    let (rotation, flipped) = match tile >> 29 {
                        0b000 => (0, false),
                        0b101 => (1, false),
                        0b110 => (2, false),
                        0b011 => (3, false),
                        0b100 => (0, true),
                        0b111 => (1, true),
                        0b010 => (2, true),
                        0b001 => (3, true),
                        _ => unreachable!()
                    };
                    if let Some(&tile) = tile_lookup.get(&(tile & 0x0fffffff)) {
                        let handle = world.add(Isometry2::new(Vector2::new(x as f64 * tiled_map.tile_width as f64, y as f64 * tiled_map.tile_height as f64), nalgebra::zero()), tiles.get(tile).hitbox.clone(), map_groups, contacts_query, ());
                        print!("{:?}: {}:({},{},{}), ", (x as f64 * tiled_map.tile_width as f64, y as f64 * tiled_map.tile_height as f64), tile, rotation, flipped, handle.0);
                        map.insert((x as i32, y as i32), MapTile {
                            tile,
                            rotation,
                            flipped
                        });
                    }
                }
                println!();
            }
        }
        let mut objects = HashMap::new();
        for group in tiled_map.object_groups {
            for object in group.objects {
                let tile_id = tile_lookup[&(object.gid & 0x0fffffff)];
                let tile = tiles.get(tile_id);
                let rotation = (((object.rotation / 90.0).round() as i32 % 4) + 4) as usize % 4;
                let (x, y) = match rotation {
                    0 => (object.x, object.y),
                    1 => (object.x, object.y + tile.height as f32),
                    2 => (object.x - tile.width as f32, object.y + tile.height as f32),
                    3 => (object.x - tile.width as f32, object.y),
                    _ => unreachable!()
                };
                let flipped = (object.gid >> 31) == 1;
                let x = x as f64;
                let y = y as f64 - tile.height as f64;
                let handle = world.add(Isometry2::new(Vector2::new(x, y), nalgebra::zero()), tile.hitbox.clone(), object_groups, contacts_query, ());
                let mut object = Object::new(tiles.lua, tile, x, y, rotation, flipped);
                objects.insert(handle, (tile_id, object));
            }
        }
        
        let color = if let Some(color) = tiled_map.background_colour {
            let color = palette::Srgb::from_pixel(&[color.red, color.green, color.blue]).into_linear();
            [
                color.red,
                color.green,
                color.blue,
                1.0
            ]
        } else {
            [0.0, 0.0, 0.0, 1.0]
        };
        Map {
            map,
            objects,
            color,
            world,
        }
    }
}

struct Tiles<'a> {
    tiles: Vec<Tile<'a>>,
    offsets: HashMap<String, usize>,
    lua: &'a Lua,
}

impl<'a> Tiles<'a> {
    fn new(lua: &'a Lua) -> Tiles<'a> {
        Tiles {
            tiles: Vec::new(),
            offsets: HashMap::new(),
            lua,
        }
    }
    fn load<R: gfx::Resources, F: gfx::Factory<R>>(&mut self, graphics: &mut Graphics<R, F>, tileset: Tileset) -> HashMap<u32, usize> {
        use std::fs::File;
        use tiled::PropertyValue::*;
        let first_gid = tileset.first_gid;
        if let Some(offset) = self.offsets.get(&tileset.name) {
            return tileset.tiles.into_iter().enumerate().map(|(i, tile)| (tile.id + first_gid, offset + i)).collect()
        }
        self.offsets.insert(tileset.name, self.tiles.len());
        let default_palette = tileset.properties.get("palette").map_or(None, |prop| match prop {
            StringValue(v) => Some(v),
            FileValue(v) => Some(v),
            _ => None
        }).expect("Tileset has no palette");

        let tiles = &mut self.tiles;
        let lua = self.lua;
        let offset = tiles.len();
        let mut mappings = HashMap::new();
        //Can't correctly set the animation frames until the real indexes are known
        let mut animations = Vec::new();
        for (i, tile) in tileset.tiles.into_iter().enumerate() {
            let palette = tile.properties.get("palette").map_or(None, |prop| match prop {
                StringValue(v) => Some(v),
                FileValue(v) => Some(v),
                _ => None
            }).unwrap_or(default_palette);
            let palette_id = tile.properties.get("palette_id").map_or(0, |prop| match prop {
                IntValue(v) => *v as usize,
                ColorValue(v) => *v as usize,
                FloatValue(v) => *v as usize,
                StringValue(v) => v.parse().unwrap_or(0),
                _ => 0
            });
            let script = tile.properties.get("script").map_or(None, |prop| match prop {
                StringValue(v) | FileValue(v) => {
                    let path = format!("assets/tiled/{}", v);
                    let mut file = File::open(&path).expect("Couldn't find script");
                    let mut contents = String::new();
                    file.read_to_string(&mut contents).expect("Failed to read file");
                    Some(lua.load(&contents, Some(&path)).expect("Failed to load script"))
                }
                _ => None
            });
            mappings.insert(first_gid + tile.id, offset + i);
            if let Some(animation) = tile.animation {
                animations.push((offset + i, animation));
            }
            let shape = ShapeHandle2::new(Cuboid2::new(Vector2::new(tile.images[0].width as f64 / 2.0, tile.images[0].height as f64 / 2.0)));
            tiles.push(Tile {
                texture: graphics.load_texture(&tile.images[0].source, &format!("assets/tiled/{}", palette), palette_id),
                animation: Vec::new(),
                width: tile.images[0].width as u32,
                height: tile.images[0].height as u32,
                script,
                hitbox: shape,
            });
        }
        for (tile, animation) in animations {
            tiles[tile].animation = animation.into_iter().map(|frame| Frame {tile: mappings[&(frame.tile_id + first_gid)], duration: frame.duration }).collect();
        }
        mappings
    }

    fn get(&self, index: usize) -> &Tile {
        &self.tiles[index]
    }
}

struct Tile<'a> {
    texture: usize,
    width: u32,
    height: u32,
    animation: Vec<Frame>,
    script: Option<rlua::Function<'a>>,
    hitbox: ShapeHandle2<f64>,
}

struct Frame {
    tile: usize,
    duration: u32,
}

struct Graphics<R: gfx::Resources, F: gfx::Factory<R>> {
    factory: F,
    textures: IndexMap<String, Texture<R>>,
    palettes: IndexMap<String, Palettes<R>>,
}

impl<R: gfx::Resources, F: gfx::Factory<R>> Graphics<R, F> {
    fn new(factory: F) -> Graphics<R, F> {
        Graphics {
            factory,
            textures: IndexMap::new(),
            palettes: IndexMap::new(),
        }
    }

    fn load_texture(&mut self, filename: &str, palette: &str, palette_id: usize) -> usize {
        let palette_index = self.load_palette(palette);
        let factory = &mut self.factory;
        let palette = self.palettes.get_index(palette_index).unwrap().1;
        let textures = &mut self.textures;
        let entry = textures.entry(filename.to_string());
        let index = entry.index();
        entry.or_insert_with(|| Texture::load(factory, palette, palette_index, filename, palette_id));
        index
    }

    fn load_palette(&mut self, filename: &str) -> usize {
        let factory = &mut self.factory;
        let entry = self.palettes.entry(filename.to_string());
        let index = entry.index();
        entry.or_insert_with(|| Palettes::load(factory, filename));
        index
    }

    fn get_texture(&self, texture: usize) -> &Texture<R> {
        self.textures.get_index(texture).unwrap().1
    }

    fn get_palette(&self, palette: usize) -> &Palettes<R> {
        self.palettes.get_index(palette).unwrap().1
    }
}

struct Texture<R: gfx::Resources> {
    vertex_buffers: [gfx::handle::Buffer<R, Vertex>; 4],
    texture: gfx::handle::ShaderResourceView<R, u32>,
    palette: usize,
}

impl<R: gfx::Resources> Texture<R> {
    fn load<F: gfx::Factory<R>>(factory: &mut F, palettes: &Palettes<R>, palette_index: usize, path: &str, palette_id: usize) -> Texture<R> {
        let path = format!("assets/images/{}", path);
        let img = image::open(&path).expect(&path).to_rgba();
        let (width, height) = img.dimensions();
        let kind = gfx::texture::Kind::D2(width as u16, height as u16, gfx::texture::AaMode::Single);
        let mut missing = HashMap::new();
        let palette_lookup = &palettes.palettes[palette_id].0;
        let mut data = Vec::new();
        for (x, y, pixel) in img.enumerate_pixels() {
            if pixel[3] == 0xFF {
                if let Some(index) = palette_lookup.get(&pixel) {
                    data.push(*index as u8)
                } else {
                    missing.entry(pixel.clone()).or_insert_with(|| Vec::new()).push((x, y));
                    data.push(palette_lookup.len() as u8)
                }
            } else {
                data.push(palette_lookup.len() as u8)
            }
        }
        for pixel in missing {
            println!("Missing color: {:?}", pixel);
        }
        let (_, view) = factory.create_texture_immutable_u8::<TextureColorFormat>(kind, gfx::texture::Mipmap::Provided, &[&data]).unwrap();

        let width = width as f32;
        let height = height as f32;
        println!("{}: {}, {}", path, width, height);

        let vertex_buffers = [
            factory.create_vertex_buffer(&[
                Vertex { pos: [width, 0.0], uv: [1.0, 0.0] },
                Vertex { pos: [0.0, 0.0], uv: [0.0, 0.0] },
                Vertex { pos: [0.0, height], uv: [0.0, 1.0] },
                Vertex { pos: [width, height], uv: [1.0, 1.0] },
            ]),
            factory.create_vertex_buffer(&[
                Vertex { pos: [height, 0.0], uv: [0.0, 0.0] },
                Vertex { pos: [0.0, 0.0], uv: [0.0, 1.0] },
                Vertex { pos: [0.0, width], uv: [1.0, 1.0] },
                Vertex { pos: [height, width], uv: [1.0, 0.0] },
            ]),
            factory.create_vertex_buffer(&[
                Vertex { pos: [width, 0.0], uv: [0.0, 1.0] },
                Vertex { pos: [0.0, 0.0], uv: [1.0, 1.0] },
                Vertex { pos: [0.0, height], uv: [1.0, 0.0] },
                Vertex { pos: [width, height], uv: [0.0, 0.0] },
            ]),
            factory.create_vertex_buffer(&[
                Vertex { pos: [height, 0.0], uv: [1.0, 1.0] },
                Vertex { pos: [0.0, 0.0], uv: [1.0, 0.0] },
                Vertex { pos: [0.0, width], uv: [0.0, 0.0] },
                Vertex { pos: [height, width], uv: [0.0, 1.0] },
            ]),
        ];
        Texture {
            vertex_buffers,
            texture: view,
            palette: palette_index,
        }
    }
}

struct Palettes<R: gfx::Resources> {
    palettes: Vec<(HashMap<image::Rgba<u8>, usize>, ShaderResourceView<R, [f32; 4]>)>
}

impl<R: gfx::Resources> Palettes<R> {
    fn load<F: gfx::Factory<R>>(factory: &mut F, filename: &str) -> Palettes<R> {
        let img = image::open(filename).expect("Unable to open palette image");
        let img = img.as_rgba8().expect("Unable to convert to RGBA image");
        let mut palettes = Vec::new();
        for y in 0..img.height() {
            let mut palette_lookup = HashMap::new();
            let mut palette = Vec::new();
            for x in 0..img.width() {
                let color = *img.get_pixel(x, y);
                palette_lookup.insert(color, x as usize);
                palette.push(color.data[0]);
                palette.push(color.data[1]);
                palette.push(color.data[2]);
                palette.push(color.data[3]);
            }
            palette.push(0);
            palette.push(0);
            palette.push(0);
            palette.push(0);
            let (_, palette_texture) = factory.create_texture_immutable_u8::<ColorFormat>(gfx::texture::Kind::D1(64), gfx::texture::Mipmap::Provided, &[palette.as_slice()]).unwrap();
            palettes.push((palette_lookup, palette_texture));
        }
        Palettes {
            palettes
        }
    }
    fn get(&self, index: usize) -> ShaderResourceView<R, [f32; 4]> {
        self.palettes[index].1.clone()
    }
}

