use std::collections::{HashMap, HashSet};

use bevy::{ecs::event::Events, input::mouse::MouseWheel, prelude::*};
use bevy_inspector_egui::{RegisterInspectable, WorldInspectorPlugin};
use bevy_mod_picking::*;
use std::hash::Hash;

mod components;
use components::*;
use rand::prelude::SliceRandom;

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
        .init_resource::<TileSelection>()
        .init_resource::<Rules>()
        .add_startup_system(setup)
        .register_inspectable::<Coordinates>()
        .register_inspectable::<RuleTileTag>()
        .register_inspectable::<PaletteTag>()
        .register_inspectable::<Orientation>()
        .register_inspectable::<Tile>()
        .register_inspectable::<OptionalTile>()
        .add_system(apply_coordinate)
        .add_system(animate_light_direction)
        .add_system(pick_tile)
        .add_system(pick_draw_tile)
        .add_system(draw_rules)
        .add_system(draw_map)
        .add_system(collapse)
        .add_system_to_stage(CoreStage::PostUpdate, on_mouse_wheel)
        .add_system_to_stage(CoreStage::PostUpdate, palette_select)
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
                                palette
                                    .spawn_bundle(PbrBundle {
                                        material: models.pick_mat.clone(),
                                        mesh: models.pick_mesh.clone(),
                                        ..Default::default()
                                    })
                                    .insert_bundle(PickableBundle::default())
                                    .insert_bundle((
                                        Name::from(format!("tile proto {i}")),
                                        Coordinates::new(i as i32, -1),
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
                            for x in 0..16 {
                                for y in 0..16 {
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

fn pick_draw_tile(
    mut query: Query<(&mut DrawTile, &OptionalTile, &Hover)>,
    selection: Res<TileSelection>,
) {
    for (mut draw_tile, map_tile, hover) in query.iter_mut() {
        match hover.hovered() {
            true => {
                let tile = OptionalTile::new(selection.make_tile());
                if draw_tile.tile != tile {
                    draw_tile.tile = tile;
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

fn draw_rules(
    query: Query<(Entity, &DrawTile), Changed<DrawTile>>,
    mut commands: Commands,
    models: Res<ModelAssets>,
    rules: Res<Rules>,
) {
    for (entity, draw_tile) in query.iter() {
        let mut entity = commands.entity(entity);
        entity.despawn_descendants();

        if let Some(tile) = &draw_tile.tile.tile {
            entity.with_children(|parent| {
                let prototype = &rules.prototypes[tile.prototype_index];
                let model = prototype.model.clone();
                let transform = Transform::from_rotation(tile.orientation.clone().into())
                    .with_translation(Vec3::new(0.0, 0.2, 0.0));

                parent
                    .spawn_bundle((transform, GlobalTransform::default()))
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

fn draw_map(
    query: Query<(Entity, &TileSuperposition), Changed<TileSuperposition>>,
    mut commands: Commands,
    models: Res<ModelAssets>,
    rules: Res<Rules>,
) {
    for (entity, multi_tile) in query.iter() {
        let mut entity = commands.entity(entity);
        entity.despawn_descendants();

        match multi_tile.tiles.len() {
            0 => {
                entity.with_children(|tile| {
                    tile.spawn_bundle(PbrBundle {
                        mesh: models.impossible_mesh.clone(),
                        material: models.impossible_mat.clone(),
                        ..Default::default()
                    });
                });
            }
            1 => {
                let tile = multi_tile.tiles.iter().next().unwrap();
                let prototype = &rules.prototypes[tile.prototype_index];
                let model = prototype.model.clone();
                let transform = Transform::from_rotation(tile.orientation.clone().into());
                entity.with_children(|tile| {
                    tile.spawn_bundle(TransformBundle::from_transform(transform))
                        .with_children(|tile| {
                            tile.spawn_scene(model);
                        });
                });
            }
            _ => {
                entity.with_children(|tile| {
                    tile.spawn_bundle(PbrBundle {
                        mesh: models.undecided_mesh.clone(),
                        material: models.undecided_mat.clone(),
                        ..Default::default()
                    });
                });
            }
        }
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
    mut query: Query<(&mut OptionalTile, &Hover)>,
    selection: Res<TileSelection>,
    mouse_button_input: Res<Input<MouseButton>>,
    mut event_writer: EventWriter<RulesNeedUpdateEvent>,
) {
    let new_tile;
    if mouse_button_input.pressed(MouseButton::Left) {
        new_tile = selection.make_tile();
    } else if mouse_button_input.pressed(MouseButton::Right) {
        new_tile = None;
    } else {
        return;
    }
    let new_tile = OptionalTile::new(new_tile);

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

/// Safe tile get from indexes
fn get_tile_prototype(map: &Vec<Vec<OptionalTile>>, coordinates: &Coordinates) -> Option<Tile> {
    if coordinates.x < 0 || coordinates.y < 0 {
        return None;
    }
    let line = map.get(coordinates.x as usize)?;
    let tile = line.get(coordinates.y as usize)?;
    tile.tile.clone()
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

fn expand_with_rotations(
    constraints: &HashMap<Tile, Constraints>,
    prototypes: &Vec<TilePrototype>,
) -> HashMap<Tile, Constraints> {
    let mut expanded = HashMap::<Tile, Constraints>::new();

    for (tile, tile_constraints) in constraints.iter() {
        let prototype = &prototypes[tile.prototype_index];
        for tile_rotations in 0..Orientation::values().len() as i32 {
            let rotated_tile = prototype.make_rotated_tile(tile.orientation, tile_rotations);
            let new_variant_constraints_entry = expanded.entry(rotated_tile).or_default();

            for (orientation, allowed_values) in tile_constraints.constraints.iter() {
                let new_constraints_entry: &mut HashSet<Tile> = new_variant_constraints_entry
                    .constraints
                    .entry(orientation.rotated(tile_rotations))
                    .or_default();
                for allowed_tile in allowed_values.iter() {
                    let prototype = &prototypes[allowed_tile.prototype_index];
                    let rotated_allowed_tile =
                        prototype.make_rotated_tile(allowed_tile.orientation, tile_rotations);
                    new_constraints_entry.insert(rotated_allowed_tile);
                }
            }
        }
    }

    info!("{:#?}", expanded);

    expanded
}

fn intersection<T: Eq + Hash>(a: HashSet<T>, b: &HashSet<T>) -> HashSet<T> {
    a.into_iter().filter(|e| b.contains(e)).collect()
}

fn collapse(
    mut rules: ResMut<Rules>,
    rules_query: Query<(&OptionalTile, &Coordinates), With<RuleTileTag>>,
    mut event_reader: EventReader<RulesNeedUpdateEvent>,
    mut tiles_query: Query<(Entity, &mut TileSuperposition, &Connectivity)>,
) {
    let mut rng = rand::thread_rng();

    if !event_reader.is_empty() {
        info!("Rules changed, clearing");
        for _ in event_reader.iter() {}
        // Rule change

        // Read the rule map
        let rule_width = 16;
        let rule_height = 16;
        let mut rule_tiles = vec![vec![OptionalTile::default(); rule_width]; rule_height];
        for (tile, coordinates) in rules_query.iter() {
            rule_tiles[coordinates.x as usize][coordinates.y as usize] = tile.clone();
        }

        // Store the rule connectivities as constraints
        rules.constraints = HashMap::<Tile, Constraints>::new();
        for x in 0..rule_width {
            for y in 0..rule_height {
                let tile = &rule_tiles[x][y];
                let coords = Coordinates::new(x as i32, y as i32);
                if let Some(tile) = &tile.tile {
                    let constraints = &mut rules
                        .constraints
                        .entry(tile.clone())
                        .or_default()
                        .constraints;

                    for orientation in Orientation::values() {
                        let neighbour_coords = orientation.offset(&coords);
                        let neighbour_tile = get_tile_prototype(&rule_tiles, &neighbour_coords);
                        if let Some(neighbour_tile) = neighbour_tile {
                            constraints
                                .entry(orientation)
                                .or_default()
                                .insert(neighbour_tile);
                        }
                    }
                }
            }
        }
        rules.constraints = expand_with_rotations(&rules.constraints, &rules.prototypes);

        // Reset to every possibilities on rule change
        let mut possible_tiles = HashSet::new();
        for tile in rules.constraints.keys() {
            possible_tiles.insert(tile.clone());
        }
        for (_, mut multi_tile_prototype, _) in tiles_query.iter_mut() {
            multi_tile_prototype.tiles = possible_tiles.clone();
        }
    }

    // Store locally the state
    let mut entity_indexes = HashMap::<Entity, usize>::new();
    let mut entities = Vec::new();
    let mut index: usize = 0;
    for (entity, _, _) in tiles_query.iter() {
        entity_indexes.insert(entity, index);
        entities.push(entity);
        index += 1;
    }
    let count = index;

    let mut waves = Vec::new();
    let mut connectivities = Vec::new();
    for (_, multi_line_prototype, connectivity) in tiles_query.iter() {
        waves.push(multi_line_prototype.tiles.clone());
        let mut connectivity_by_index = HashMap::new();
        for (orientation, entity) in connectivity.connectivity.iter() {
            connectivity_by_index.insert(*orientation, *entity_indexes.get(entity).unwrap());
        }
        connectivities.push(connectivity_by_index);
    }

    // Find the smallest > 1 entropy
    let mut min_entropy_entities = Vec::new();
    let mut min_entropy = usize::MAX;

    for i in 0..count {
        let entropy = waves[i].len();
        if entropy < min_entropy && entropy > 1 {
            min_entropy = entropy;
            min_entropy_entities.clear();
        }

        if entropy == min_entropy {
            min_entropy_entities.push(i);
        }
    }
    let min_entropy_entity = min_entropy_entities.choose(&mut rng);

    if let Some(min_entropy_entity) = min_entropy_entity {
        let min_entropy_entity = *min_entropy_entity;
        // Observe the tile with the smallest entropy
        observe(&mut waves[min_entropy_entity], &mut rng);

        // Propagate
        let mut need_propagation = HashSet::<usize>::new();
        need_propagation.insert(min_entropy_entity);
        while !need_propagation.is_empty() {
            // Pop an entity needing propagation
            let propagating_entity = need_propagation.iter().next().cloned().unwrap();
            need_propagation.take(&propagating_entity).unwrap();

            // Get all its allowed values and its connectivity
            let propagating_wave = waves[propagating_entity].clone();

            if propagating_wave.is_empty() {
                // Impossible to solve
                // Avoid propagating it everywhere
                continue;
            }

            let propagating_connectivity = connectivities[propagating_entity].clone();

            // Find its neighbours
            for orientation in Orientation::values() {
                if let Some(neighbour) = propagating_connectivity.get(&orientation) {
                    // Sum all the possible values for this neighbour given its own allowed values
                    let mut all_allowed_neighbour = HashSet::<Tile>::new();
                    for value in &propagating_wave {
                        let rule_constraints = rules
                            .constraints
                            .get(value)
                            .unwrap()
                            .constraints
                            .get(&orientation);
                        if let Some(allowed_neighbour) = rule_constraints {
                            all_allowed_neighbour.extend(allowed_neighbour);
                        }
                    }

                    // Intersect the previous list of allowed values with the new constraints
                    let new_allowed_values =
                        intersection(all_allowed_neighbour, &waves[*neighbour]);

                    // If impacted, update the tile and add it to the list needing propagation
                    if &new_allowed_values != &waves[*neighbour] {
                        need_propagation.insert(*neighbour);
                        waves[*neighbour].clear();
                        waves[*neighbour].extend(new_allowed_values.iter());
                    }
                }
            }
        }
    }

    // Apply the result to the entities
    for i in 0..count {
        let mut multitiles = tiles_query
            .get_component_mut::<TileSuperposition>(entities[i])
            .unwrap();
        if multitiles.tiles != waves[i] {
            multitiles.tiles = waves[i].clone();
        }
    }
}

fn observe(multi_tile_prototype: &mut HashSet<Tile>, rng: &mut rand::prelude::ThreadRng) {
    let tile_vec: Vec<&Tile> = multi_tile_prototype.iter().collect();
    let observed = *tile_vec.choose(rng).unwrap().clone();
    multi_tile_prototype.clear();
    multi_tile_prototype.insert(observed.clone());
}

fn palette_select(
    mut events: EventReader<PickingEvent>,
    mut selection: ResMut<TileSelection>,
    palette_query: Query<&Tile, With<PaletteTag>>,
    rules: Res<Rules>,
) {
    for event in events.iter() {
        match event {
            PickingEvent::Clicked(e) => {
                match palette_query.get(*e) {
                    Ok(e) => {
                        selection.prototype = Some(rules.prototypes[e.prototype_index].clone())
                    }
                    Err(_) => (),
                };
            }
            _ => (),
        }
    }
}

fn on_mouse_wheel(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut selection: ResMut<TileSelection>,
) {
    for event in mouse_wheel_events.iter() {
        selection.rotation += event.y as i32;
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
