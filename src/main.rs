use std::collections::HashMap;

use bevy::prelude::*;
use bevy_inspector_egui::{InspectorPlugin, WorldInspectorPlugin};
use bevy_mod_picking::*;

mod components;
use components::*;
mod display;
mod input;
mod wcf;
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(WorldInspectorPlugin::new())
        .add_plugin(InspectorPlugin::<Tuning>::new())
        .add_plugins(DefaultPickingPlugins)
        .add_plugin(components::ComponentsPlugin)
        .add_plugin(wcf::WCFPlugin)
        .add_plugin(display::DisplayPlugin)
        .add_plugin(input::InputPlugin)
        .add_startup_system(setup)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .run();
}

fn setup(mut commands: Commands, rules: Res<Rules>, models: Res<ModelAssets>) {
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

    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(0.0, 20.0, 20.0)
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
            ..default()
        })
        .insert_bundle(PickingCameraBundle::default())
        .with_children(|camera| {
            let rules_width = 16;
            let rules_height = 16;
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
                            for i in 0..rules.prototypes.len() {
                                let prototype = &rules.prototypes[i];
                                let model = prototype.model.clone();
                                let x = i as i32 % rules_width;
                                let y = -(i as i32 / rules_height) - 2;
                                palette
                                    .spawn_bundle(PbrBundle {
                                        material: models.pick_mat.clone(),
                                        mesh: models.pick_mesh.clone(),
                                        ..Default::default()
                                    })
                                    .insert_bundle(PickableBundle::default())
                                    .insert_bundle((
                                        Name::from(format!("tile proto {i}")),
                                        Coordinates::new(x, y),
                                        Tile::new(i, Orientation::North),
                                        PaletteTag {},
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
                            for x in 0..rules_width {
                                for y in 0..rules_height {
                                    rule_map
                                        .spawn_bundle(PbrBundle {
                                            material: models.pick_mat.clone(),
                                            mesh: models.pick_mesh.clone(),
                                            ..Default::default()
                                        })
                                        .insert_bundle((
                                            Name::from(format!("{x}:{y}")),
                                            Coordinates::new(x as i32, y as i32),
                                            OptionalTile::default(),
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
    let width = rules.width;
    let height = rules.height;
    let mut map_entities = vec![vec![Entity::from_raw(0); height]; width];
    commands
        .spawn_bundle(TransformBundle::from_transform(Transform::from_xyz(
            -((width / 2) as f32),
            0.0,
            -((height / 2) as f32),
        )))
        .insert(Name::from("world_map"))
        .with_children(|rule_map| {
            for x in 0..width {
                for y in 0..height {
                    let entity = rule_map
                        .spawn_bundle(TransformBundle::default())
                        .insert_bundle((
                            Name::from(format!("{x}:{y}")),
                            Coordinates::new(x as i32, y as i32),
                            TileSuperposition::default(),
                            TileSuperpositionHistory::default(),
                        ))
                        .id();
                    map_entities[x][y] = entity;
                }
            }
        });

    // Compute connectivity
    for x in 0..width {
        for y in 0..height {
            let mut entity = commands.entity(map_entities[x][y]);
            let coord = Coordinates::new(x as i32, y as i32);
            let mut connectivity = HashMap::new();
            for orientation in Orientation::values() {
                let neighbour_coords = orientation.offset(&coord);
                if let Some(e) = get_tile_entity(&map_entities, &neighbour_coords) {
                    connectivity.insert(orientation, e);
                }
            }
            entity.insert(Connectivity { connectivity });
        }
    }
}

/// Safe tile get from indexes
fn get_tile_entity(map: &Vec<Vec<Entity>>, coordinates: &Coordinates) -> Option<Entity> {
    if coordinates.x < 0 || coordinates.y < 0 {
        return None;
    }
    let line = map.get(coordinates.x as usize)?;
    let tile = line.get(coordinates.y as usize)?;
    Some(tile.clone())
}
