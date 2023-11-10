use std::{
    collections::HashMap,
    f32::consts::PI,
    path::{Path, PathBuf},
    time::Duration,
};

use bevy::{
    prelude::*,
    sprite::MaterialMesh2dBundle,
    utils::petgraph::{
        self,
        stable_graph::NodeIndex,
        visit::{Bfs, Walker},
        Graph,
    },
    window::PrimaryWindow,
};
use bevy_prototype_lyon::{path, prelude::*};
use bevy_tweening::{lens::ColorMaterialColorLens, *};
use rand::Rng;

use crate::generate_graph;

pub fn init(watch: impl AsRef<Path>) {
    App::new()
        // Plugins
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Call Graph".to_owned(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ShapePlugin)
        .add_plugins(TweeningPlugin)
        // Systems
        .add_systems(Startup, setup)
        .add_systems(Update, draw_edges)
        .add_systems(Update, load_graph)
        .add_systems(Update, add_node_forces)
        .add_systems(Update, update_cursor_coords)
        .add_systems(Update, draggables)
        .add_systems(Update, move_draggable_locked)
        .add_systems(Update, graph_highlights)
        .add_systems(Update, highlight)
        .add_systems(Update, bevy::window::close_on_esc)
        // Events
        .add_event::<LoadGraph>()
        // Resources
        .insert_resource(LoadPath(watch.as_ref().to_path_buf()))
        .insert_resource(CursorCoords::default())
        .insert_resource(NodeGraph::default())
        .insert_resource(ClearColor(Color::rgb_u8(25, 25, 35)))
        .run();
}

#[derive(Resource, Default)]
struct CursorCoords(Vec2);

#[derive(Resource)]
struct LoadPath(PathBuf);

#[derive(Component)]
struct Node(NodeIndex, Vec<NodeIndex>);

#[derive(Component)]
struct Draggable {
    hit_radius: f32,
}

#[derive(Component)]
struct DraggableLocked;

#[derive(Component)]
struct Edge(NodeIndex, NodeIndex);

#[derive(Component)]
struct Highlight(Color);

#[derive(Event)]
struct LoadGraph;

#[derive(Resource, Default, Debug)]
struct NodeGraph(Graph<Entity, ()>);

impl NodeGraph {
    fn get_node(&self, node: NodeIndex) -> Entity {
        *self.0.node_weight(node).unwrap()
    }
}

fn setup(mut commands: Commands, mut ev_load_graph: EventWriter<LoadGraph>) {
    commands.spawn(Camera2dBundle::default());
    ev_load_graph.send(LoadGraph);
}

fn load_graph(
    mut commands: Commands,
    mut ev_load_graph: EventReader<LoadGraph>,
    load_path: Res<LoadPath>,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut res_graph: ResMut<NodeGraph>,

    old_nodes: Query<Entity, With<Node>>,
) {
    for _ in ev_load_graph.read() {
        old_nodes.for_each(|e| {
            commands.entity(e).despawn_recursive();
        });

        let input = std::fs::read_to_string(&load_path.0).unwrap();
        let graph = generate_graph::generate_graph(&input, &load_path.0.to_string_lossy()).unwrap();
        let mut id_lookups = HashMap::new();
        let mut rng = rand::thread_rng();
        for (i, node) in graph.keys().enumerate() {
            let x = rng.gen_range((-250.)..250.);
            let y = rng.gen_range((-250.)..250.);
            let id = res_graph.0.add_node(
                commands
                    .spawn((
                        MaterialMesh2dBundle {
                            mesh: meshes.add(shape::Circle::new(30.).into()).into(),
                            material: materials
                                .add(ColorMaterial::from(Color::BLUE.with_s(0.3).with_l(0.5))),
                            transform: Transform::from_xyz(x, y, i as f32),
                            ..default()
                        },
                        Draggable { hit_radius: 30. },
                    ))
                    .with_children(|parent| {
                        let len = node.len();
                        parent.spawn(Text2dBundle {
                            text: Text::from_section(
                                node,
                                TextStyle {
                                    font_size: 50.,
                                    color: Color::WHITE,
                                    ..default()
                                },
                            )
                            .with_alignment(TextAlignment::Center),
                            transform: Transform::from_xyz((len / 2) as f32 * 15., 70., 1.),
                            ..default()
                        });
                    })
                    .id(),
            );
            id_lookups.insert(node, id);
        }
        for (node, neighbors) in &graph {
            let id = id_lookups[&node];
            let neighbor_ids = neighbors.iter().map(|n| id_lookups[n]).collect::<Vec<_>>();
            commands
                .entity(*res_graph.0.node_weight(id).unwrap())
                .insert(Node(id, neighbor_ids));
            for neighbor in neighbors {
                commands.spawn(Edge(id, id_lookups[neighbor]));
                res_graph.0.add_edge(id, id_lookups[neighbor], ());
            }
        }
    }
}

fn draw_edges(
    mut commands: Commands,
    nodes: Query<&Transform, With<Node>>,
    edges: Query<(&Edge, Entity)>,
    graph: Res<NodeGraph>,
) {
    edges
        .iter()
        .for_each(|(_, e)| commands.entity(e).despawn_recursive());
    for edge in edges.iter().map(|e| e.0) {
        let color: Color = Color::DARK_GRAY;

        if edge.0 == edge.1 {
            let node = nodes
                .get_component::<Transform>(graph.get_node(edge.0))
                .unwrap();
            let mut path = path::PathBuilder::new();
            path.move_to(node.translation.truncate());
            path.arc(
                node.translation.truncate() + Vec2::Y * 40.,
                Vec2::new(30., 40.),
                20.,
                PI,
            );
            let triangle = shapes::RegularPolygon {
                sides: 3,
                feature: shapes::RegularPolygonFeature::Radius(5.),
                ..default()
            };
            commands
                .spawn((
                    (
                        ShapeBundle {
                            path: GeometryBuilder::build_as(&path.build()),
                            spatial: SpatialBundle {
                                transform: Transform::from_xyz(0., 0., -1.),
                                ..default()
                            },
                            ..default()
                        },
                        Stroke::new(color, 1.),
                    ),
                    Edge(edge.0, edge.1),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        ShapeBundle {
                            path: GeometryBuilder::build_as(&triangle),
                            spatial: SpatialBundle {
                                transform: Transform {
                                    translation: node.translation + Vec3::Y * 24. + Vec3::X * -28.,
                                    rotation: Quat::from_rotation_z(10.),
                                    ..default()
                                },
                                ..default()
                            },
                            ..default()
                        },
                        Fill::color(color),
                        Stroke::new(color, 1.),
                    ));
                });
            continue;
        }

        let head = nodes
            .get_component::<Transform>(graph.get_node(edge.0))
            .unwrap();
        let tail = nodes
            .get_component::<Transform>(graph.get_node(edge.1))
            .unwrap();
        if head.translation == tail.translation {
            continue;
        }
        let line = shapes::Line(head.translation.truncate(), tail.translation.truncate());
        let triangle = shapes::RegularPolygon {
            sides: 3,
            feature: shapes::RegularPolygonFeature::Radius(5.),
            ..default()
        };
        // Create the corresponding vector for the line
        let line_vec = Vec2::new(line.1.x - line.0.x, line.1.y - line.0.y).normalize();
        let triangle_pos = tail.translation.truncate() - line_vec * 35.;
        let direction = (tail.translation - triangle_pos.extend(0.)).normalize();
        let triangle_rot = Quat::from_rotation_z(direction.y.atan2(direction.x) - 10.);

        commands
            .spawn((
                (
                    ShapeBundle {
                        path: GeometryBuilder::build_as(&line),
                        spatial: SpatialBundle {
                            // Lines should be drawn behind nodes
                            transform: Transform::from_xyz(0., 0., -1.),
                            ..default()
                        },
                        ..default()
                    },
                    Fill::color(color),
                    Stroke::new(color, 1.),
                ),
                Edge(edge.0, edge.1),
            ))
            .with_children(|parent| {
                parent.spawn((
                    ShapeBundle {
                        path: GeometryBuilder::build_as(&triangle),
                        spatial: SpatialBundle {
                            transform: Transform {
                                translation: triangle_pos.extend(0.),
                                rotation: triangle_rot,
                                ..default()
                            },
                            ..default()
                        },
                        ..default()
                    },
                    Fill::color(color),
                    Stroke::new(color, 1.),
                ));
            });
    }
}

fn add_node_forces(
    edges: Query<&Edge>,
    mut nodes: Query<&mut Transform, With<Node>>,
    graph: Res<NodeGraph>,
) {
    // Apply strong, constrained attraction between connected nodes
    const STRENGTH: f32 = 200.;
    for edge in &edges {
        if edge.0 == edge.1 {
            continue;
        }

        let head = graph.get_node(edge.0);
        let tail = graph.get_node(edge.1);
        let tail_pos = nodes.get_component::<Transform>(tail).unwrap().translation;
        let mut head_transform = nodes.get_component_mut::<Transform>(head).unwrap();
        let force = calc_force(
            tail_pos.truncate(),
            head_transform.translation.truncate(),
            STRENGTH,
        );
        head_transform.translation += (force * 2.).extend(0.);
    }
    for edge in &edges {
        if edge.0 == edge.1 {
            continue;
        }

        let head = graph.get_node(edge.1);
        let tail = graph.get_node(edge.0);
        let tail_pos = nodes.get_component::<Transform>(tail).unwrap().translation;
        let mut t1 = nodes.get_component_mut::<Transform>(head).unwrap();
        let force = calc_force(tail_pos.truncate(), t1.translation.truncate(), STRENGTH);
        t1.translation += force.extend(0.);
    }

    // Apply weak repulsion between nodes
    let translations = nodes.iter().map(|t| t.translation).collect::<Vec<_>>();
    const DISTANCE: f32 = 80.;
    for t in translations {
        for mut t2 in nodes.iter_mut() {
            if t == t2.translation || t.distance(t2.translation) > DISTANCE {
                continue;
            }
            let force = calc_force(t2.translation.truncate(), t.truncate(), DISTANCE);
            t2.translation += -(force * 0.5).extend(0.);
        }
    }
}

fn calc_force(p: Vec2, q: Vec2, strength: f32) -> Vec2 {
    let diff = p - q;
    let dist = diff.length();
    diff.normalize() * (dist - strength) / strength
}

fn draggables(
    mut commands: Commands,
    ts: Query<(&Transform, Entity, &Draggable)>,
    draggables: Query<With<DraggableLocked>>,
    buttons: Res<Input<MouseButton>>,
    cursor_coords: Res<CursorCoords>,
) {
    let has_dragged = draggables.iter().count() >= 1;
    if buttons.pressed(MouseButton::Left) && !has_dragged {
        for (t, entity, draggable) in &ts {
            let collision =
                t.translation.truncate().distance(cursor_coords.0) < draggable.hit_radius;
            if collision {
                commands.entity(entity).insert(DraggableLocked);
            }
        }
    }
    if buttons.just_released(MouseButton::Left) {
        for (_, entity, _) in &ts {
            commands.entity(entity).remove::<DraggableLocked>();
        }
    }
}

fn move_draggable_locked(
    mut draggables: Query<&mut Transform, With<DraggableLocked>>,
    cursor_coords: Res<CursorCoords>,
) {
    let Ok(mut t) = draggables.get_single_mut() else {
        return;
    };
    let old = t.translation.z;
    t.translation = cursor_coords.0.extend(old);
}

fn highlight(
    mut commands: Commands,
    highlights: Query<(&Highlight, &Handle<ColorMaterial>, Entity), Changed<Highlight>>,

    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (highlight, mat, entity) in &mut highlights.iter() {
        let material = materials.get_mut(mat).unwrap();
        let tween = Tween::new(
            // Use a quadratic easing on both endpoints.
            EaseFunction::QuadraticInOut,
            Duration::from_millis(500),
            ColorMaterialColorLens {
                start: material.color,
                end: highlight.0,
            },
        );
        commands.entity(entity).insert(AssetAnimator::new(tween));
    }
}

fn update_cursor_coords(
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mut cursor_coords: ResMut<CursorCoords>,
) {
    let (camera, camera_transform) = camera.single();
    let window = window.single();

    let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    else {
        return;
    };

    cursor_coords.0 = world_position;
}

fn graph_highlights(mut commands: Commands, keys: Res<Input<KeyCode>>, graph: Res<NodeGraph>) {
    let any_pressed = keys.just_pressed(KeyCode::R)
        || keys.just_pressed(KeyCode::Key1)
        || keys.just_pressed(KeyCode::Key2)
        || keys.just_pressed(KeyCode::Key3);

    if any_pressed {
        // Remove all graph_highlights
        for node in graph.0.node_indices() {
            let node = graph.get_node(node);
            commands
                .entity(node)
                .insert(Highlight(Color::BLUE.with_s(0.3).with_l(0.5)));
        }
    }

    match (
        keys.just_pressed(KeyCode::Key1),
        keys.just_pressed(KeyCode::Key2),
        keys.just_pressed(KeyCode::Key3),
    ) {
        // Inline candidates
        (true, _, _) => {
            for node in graph.0.node_indices() {
                if graph
                    .0
                    .neighbors_directed(node, petgraph::Direction::Incoming)
                    .count()
                    != 1
                    // Prevent self-referential nodes from being highlighted
                    || graph
                        .0
                        .neighbors(node)
                        .next()
                        .is_some_and(|neighbor| neighbor == node)
                {
                    continue;
                }
                let entity = graph.get_node(node);
                commands
                    .entity(entity)
                    .insert(Highlight(Color::AQUAMARINE.with_s(0.5)));
            }
        }
        // Dead function elimination
        (_, true, _) => {
            let components = graph_components(&graph.0);
            let mut color = Color::AQUAMARINE.with_s(0.8);
            for component in components {
                let hue = color.h();
                color.set_h(hue + 40.);
                for idx in component {
                    let node = graph.get_node(idx);
                    commands.entity(node).insert(Highlight(color));
                }
            }
        }
        // Strongly connected components
        (_, _, true) => {
            let sccs = petgraph::algo::kosaraju_scc(&graph.0);
            let mut color = Color::AQUAMARINE.with_s(0.8);
            for scc in sccs {
                let hue = color.h();
                color.set_h(hue + 20.);
                for idx in scc {
                    let node = graph.get_node(idx);
                    commands.entity(node).insert(Highlight(color));
                }
            }
        }
        _ => {}
    }
}

fn graph_components<N, E>(graph: &Graph<N, E>) -> Vec<Vec<NodeIndex>> {
    let mut components = Vec::new();
    for node in graph.node_indices() {
        if components
            .iter()
            .any(|c: &Vec<NodeIndex>| c.contains(&node))
        {
            continue;
        }

        let component = Bfs::new(&graph, node).iter(&graph).collect::<Vec<_>>();
        components.push(component);
    }
    components
}
