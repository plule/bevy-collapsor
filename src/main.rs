use bevy::{ecs::event::Events, input::mouse::MouseWheel, prelude::*, utils::HashMap};
use bevy_inspector_egui::{Inspectable, RegisterInspectable, WorldInspectorPlugin};
use bevy_mod_picking::*;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(WorldInspectorPlugin::new())
        .add_plugins(DefaultPickingPlugins)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .insert_resource(Events::<RulesNeedUpdateEvent>::default())
        .init_resource::<ModelAssets>()
        .init_resource::<SelectedTileProto>()
        .add_startup_system(setup)
        .register_inspectable::<Coordinates>()
        .register_inspectable::<RuleTileTag>()
        .register_inspectable::<Palette>()
        .register_inspectable::<Orientation>()
        .register_inspectable::<TilePrototype>()
        .register_inspectable::<SelectedTileProto>()
        .register_inspectable::<OptionalTilePrototype>()
        .add_system(apply_coordinate)
        .add_system(animate_light_direction)
        .add_system(pick_tile)
        .add_system(pick_draw_tile)
        .add_system(draw_map)
        .add_system_to_stage(CoreStage::PostUpdate, on_mouse_wheel)
        .add_system_to_stage(CoreStage::PostUpdate, on_pick_event)
        .add_system_to_stage(CoreStage::PostUpdate, read_rules)
        .run();
}

#[derive(Default)]
struct ModelAssets {
    models: Vec<Handle<Scene>>,
    up_cube_mesh: Handle<Mesh>,
    up_cube_mat: Handle<StandardMaterial>,
}

#[derive(Inspectable, Clone, Copy, PartialEq, FromPrimitive, Hash, Eq, Debug)]
enum Orientation {
    NORTH = 0,
    EST,
    SOUTH,
    WEST,
}

impl Default for Orientation {
    fn default() -> Self {
        Orientation::NORTH
    }
}

impl From<Orientation> for Quat {
    fn from(orientation: Orientation) -> Self {
        let angle = match orientation {
            Orientation::NORTH => 0.,
            Orientation::EST => -90.0_f32.to_radians(),
            Orientation::SOUTH => -180.0_f32.to_radians(),
            Orientation::WEST => -270.0_f32.to_radians(),
        };
        Quat::from_rotation_y(angle)
    }
}

impl Orientation {
    fn rotate(&mut self, amount: i32) {
        *self = FromPrimitive::from_i32(((*self as i32) + amount).rem_euclid(4)).unwrap();
    }
}

#[cfg(test)]
#[test]
fn rotate_orientation() {
    let mut orientation = Orientation::NORTH;
    orientation.rotate(-2);
    assert!(orientation == Orientation::SOUTH);
    orientation.rotate(1);
    assert!(orientation == Orientation::WEST);
}

#[derive(Default, Component, Inspectable, Clone, PartialEq, Hash, Eq, Debug)]
struct TilePrototype {
    model_index: usize,
    orientation: Orientation,
}

#[derive(Default, Component, Inspectable, Clone, PartialEq)]
struct OptionalTilePrototype {
    tile_prototype: Option<TilePrototype>,
}

#[derive(Default, Component, Inspectable, Clone, PartialEq)]
struct DrawTile {
    tile: OptionalTilePrototype,
}

impl OptionalTilePrototype {
    pub fn from_index(index: usize) -> OptionalTilePrototype {
        OptionalTilePrototype {
            tile_prototype: Some(TilePrototype {
                model_index: index,
                ..Default::default()
            }),
        }
    }
}

#[derive(Default, Inspectable)]
struct SelectedTileProto {
    tile_prototype: OptionalTilePrototype,
}

#[derive(Component, Inspectable, Default)]
struct RuleTileTag;

#[derive(Component, Inspectable)]
struct Palette {
    index: usize,
}

impl Palette {
    fn new(index: usize) -> Self {
        Self { index }
    }
}

#[derive(Component, Inspectable, Default)]
struct Coordinates {
    pub x: i32,
    pub y: i32,
}

impl Coordinates {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut models: ResMut<ModelAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const HALF_SIZE: f32 = 1.0;
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 10.0 * HALF_SIZE,
                ..default()
            },
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });

    let map = Map::new(16, 16);

    models.models = map
        .tile_models
        .iter()
        .map(|path| asset_server.load(path))
        .collect();

    models.up_cube_mesh = meshes.add(shape::Cube { size: 0.1 }.into());
    models.up_cube_mat = materials.add(Color::RED.into());

    let pick_mesh = meshes.add(Mesh::from(shape::Plane { size: 1.0 }));
    let pick_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        ..Default::default()
    });

    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(0.0, 20.0, 20.0)
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
            ..default()
        })
        .insert_bundle(PickingCameraBundle::default())
        .with_children(|camera| {
            // UI
            camera
                .spawn_bundle(TransformBundle::from(
                    Transform::identity()
                        .looking_at(Vec3::Y, Vec3::Z)
                        .with_translation(Vec3::new(-1.4, -0.2, -2.0))
                        .with_scale(Vec3::new(0.04, 0.04, 0.04)),
                ))
                .insert(Name::from("ui"))
                .with_children(|ui| {
                    // Palette
                    ui.spawn_bundle(TransformBundle::default())
                        .insert(Name::from("palette"))
                        .with_children(|palette| {
                            for i in 0..map.tile_models.len() {
                                let model = models.models[i].clone();
                                palette
                                    .spawn_bundle(PbrBundle {
                                        material: pick_mat.clone(),
                                        mesh: pick_mesh.clone(),
                                        ..Default::default()
                                    })
                                    .insert_bundle(PickableBundle::default())
                                    .insert_bundle((
                                        Name::from(format!("tile proto {i}")),
                                        Coordinates::new(2 * (i as i32), -1),
                                        Palette::new(i),
                                    ))
                                    .with_children(|tile| {
                                        tile.spawn_bundle((
                                            Transform::from_xyz(0.0, 0.2, 0.0),
                                            GlobalTransform::default(),
                                        ))
                                        .with_children(
                                            |tile| {
                                                tile.spawn_scene(model);
                                            },
                                        );
                                    });
                            }
                        });

                    // Rule map
                    ui.spawn_bundle(TransformBundle::default())
                        .insert(Name::from("rule_map"))
                        .with_children(|rule_map| {
                            for x in 0..map.width {
                                for y in 0..map.height {
                                    rule_map
                                        .spawn_bundle(PbrBundle {
                                            material: pick_mat.clone(),
                                            mesh: pick_mesh.clone(),
                                            ..Default::default()
                                        })
                                        .insert_bundle((
                                            Name::from(format!("{x}:{y}")),
                                            Coordinates::new(x as i32, y as i32),
                                            OptionalTilePrototype::default(),
                                            DrawTile::default(),
                                            RuleTileTag::default(),
                                        ))
                                        .insert_bundle(PickableBundle::default());
                                }
                            }
                        });
                });
        });

    // Generated map
    commands
        .spawn_bundle(TransformBundle::from_transform(Transform::from_xyz(
            -((map.width / 2) as f32),
            0.0,
            -((map.height / 2) as f32),
        )))
        .insert(Name::from("world_map"))
        .with_children(|rule_map| {
            for x in 0..map.width {
                for y in 0..map.height {
                    rule_map
                        .spawn_bundle(PbrBundle {
                            material: pick_mat.clone(),
                            mesh: pick_mesh.clone(),
                            ..Default::default()
                        })
                        .insert_bundle((
                            Name::from(format!("{x}:{y}")),
                            Coordinates::new(x as i32, y as i32),
                            OptionalTilePrototype::default(),
                            DrawTile::default(),
                        ))
                        .insert_bundle(PickableBundle::default());
                }
            }
        });

    commands.insert_resource(map);
}

fn pick_draw_tile(
    mut query: Query<(&mut DrawTile, &OptionalTilePrototype, &Hover)>,
    selected: Res<SelectedTileProto>,
) {
    for (mut draw_tile, map_tile, hover) in query.iter_mut() {
        match hover.hovered() {
            true => {
                if draw_tile.tile != selected.tile_prototype {
                    draw_tile.tile = selected.tile_prototype.clone();
                }
            }
            false => {
                if draw_tile.tile != *map_tile {
                    draw_tile.tile = map_tile.clone();
                }
            }
        }
    }
}

fn draw_map(
    query: Query<(Entity, &DrawTile), Changed<DrawTile>>,
    mut commands: Commands,
    models: Res<ModelAssets>,
) {
    for (entity, draw_tile) in query.iter() {
        let mut entity = commands.entity(entity);
        entity.despawn_descendants();

        if let Some(tile_prototype) = &draw_tile.tile.tile_prototype {
            entity.with_children(|tile| {
                let model = models.models[tile_prototype.model_index].clone();
                let transform = Transform::from_rotation(tile_prototype.orientation.clone().into())
                    .with_translation(Vec3::new(0.0, 0.2, 0.0));

                tile.spawn_bundle((transform, GlobalTransform::default()))
                    .with_children(|tile| {
                        tile.spawn_scene(model);
                        tile.spawn_bundle(PbrBundle {
                            material: models.up_cube_mat.clone(),
                            mesh: models.up_cube_mesh.clone(),
                            transform: Transform::from_translation(-Vec3::Z / 2.5),
                            ..Default::default()
                        });
                    });
            });
        };
    }
}

fn apply_coordinate(mut query: Query<(&mut Transform, &Coordinates), Changed<Coordinates>>) {
    for (mut transform, coordinates) in query.iter_mut() {
        transform.translation.x = coordinates.x as f32;
        transform.translation.y = 0.;
        transform.translation.z = coordinates.y as f32;
    }
}

fn pick_tile(
    mut query: Query<(&mut OptionalTilePrototype, &Hover)>,
    selected: Res<SelectedTileProto>,
    mouse_button_input: Res<Input<MouseButton>>,
    mut event_writer: EventWriter<RulesNeedUpdateEvent>,
) {
    let new_tile;
    if mouse_button_input.pressed(MouseButton::Left) {
        new_tile = selected.tile_prototype.clone();
    } else if mouse_button_input.pressed(MouseButton::Right) {
        new_tile = OptionalTilePrototype::default();
    } else {
        return;
    }

    let mut changed = false;
    for (mut map_tile, hover) in query.iter_mut() {
        if hover.hovered() && *map_tile != new_tile {
            *map_tile = new_tile.clone();
            changed = true;
        }
    }

    if changed {
        event_writer.send(RulesNeedUpdateEvent {});
    }
}

struct RulesNeedUpdateEvent {}

#[derive(Default, Debug)]
struct Constraints {
    pub top: Vec<TilePrototype>,
    pub right: Vec<TilePrototype>,
    pub down: Vec<TilePrototype>,
    pub left: Vec<TilePrototype>,
}

/// Safe tile get from indexes
fn get_tile_prototype(
    map: &Vec<Vec<OptionalTilePrototype>>,
    x: i32,
    y: i32,
) -> Option<TilePrototype> {
    if x < 0 || y < 0 {
        return None;
    }
    let line = map.get(x as usize)?;
    let tile = line.get(y as usize)?;
    tile.tile_prototype.clone()
}

fn read_rules(
    mut event_reader: EventReader<RulesNeedUpdateEvent>,
    mut map: ResMut<Map>,
    query: Query<(&OptionalTilePrototype, &Coordinates), With<RuleTileTag>>,
) {
    if event_reader.is_empty() {
        return;
    }
    for _ in event_reader.iter() {}
    info!("Updating rules");

    // Read the rule map
    let mut rule_tiles = vec![vec![OptionalTilePrototype::default(); map.height]; map.width];
    for (tile, coordinates) in query.iter() {
        rule_tiles[coordinates.x as usize][coordinates.y as usize] = tile.clone();
    }

    // Store the rule connectivities as constraints
    map.constraints = HashMap::<TilePrototype, Constraints>::new();
    for x in 0..map.width {
        for y in 0..map.height {
            let tile = &rule_tiles[x][y];
            let x = x as i32;
            let y = y as i32;
            if let Some(tile) = &tile.tile_prototype {
                let constraints = map.constraints.entry(tile.clone()).or_default();
                if let Some(top) = get_tile_prototype(&rule_tiles, x, y + 1) {
                    constraints.top.push(top);
                }
                if let Some(right) = get_tile_prototype(&rule_tiles, x + 1, y) {
                    constraints.right.push(right);
                }
                if let Some(down) = get_tile_prototype(&rule_tiles, x, y - 1) {
                    constraints.down.push(down);
                }
                if let Some(left) = get_tile_prototype(&rule_tiles, x - 1, y) {
                    constraints.left.push(left);
                }
            }
        }
    }
}

fn on_pick_event(
    mut events: EventReader<PickingEvent>,
    mut selected: ResMut<SelectedTileProto>,
    palette_query: Query<&Palette>,
) {
    for event in events.iter() {
        match event {
            PickingEvent::Selection(_) => (),
            PickingEvent::Hover(_) => (),
            PickingEvent::Clicked(e) => {
                match palette_query.get(*e) {
                    Ok(e) => selected.tile_prototype = OptionalTilePrototype::from_index(e.index),
                    Err(_) => (),
                };
            }
        }
    }
}

fn on_mouse_wheel(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut selected_tile: ResMut<SelectedTileProto>,
) {
    let selected_tile = &mut selected_tile.tile_prototype;
    if let Some(selected_tile) = &mut selected_tile.tile_prototype {
        for event in mouse_wheel_events.iter() {
            selected_tile.orientation.rotate(event.y as i32);
        }
    }
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in query.iter_mut() {
        transform.rotation = Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            time.seconds_since_startup() as f32 * std::f32::consts::TAU / 20.0,
            -std::f32::consts::FRAC_PI_4,
        );
    }
}

struct Map {
    pub tile_models: Vec<String>,
    pub width: usize,
    pub height: usize,
    pub constraints: HashMap<TilePrototype, Constraints>,
}

impl Map {
    fn new(width: usize, height: usize) -> Self {
        Self {
            tile_models: vec![
                "models/ground_grass.glb#Scene0".to_string(),
                "models/ground_pathBend.glb#Scene0".to_string(),
                "models/ground_pathCross.glb#Scene0".to_string(),
                "models/ground_pathEndClosed.glb#Scene0".to_string(),
                "models/ground_pathSplit.glb#Scene0".to_string(),
                "models/ground_pathStraight.glb#Scene0".to_string(),
            ],
            constraints: Default::default(),
            width,
            height,
        }
    }
}
