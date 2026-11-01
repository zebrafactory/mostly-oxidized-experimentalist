use avian3d::prelude::*;
use bevy::prelude::*;

const FLOOR_RADIUS: f32 = 100.0;
const FLOOR_HALF_HEIGHT: f32 = 0.5;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(Camera3d::default());

    commands.spawn((
        RigidBody::Static,
        Collider::cylinder(FLOOR_RADIUS, FLOOR_HALF_HEIGHT),
        Mesh3d(meshes.add(Cylinder::new(FLOOR_RADIUS, FLOOR_HALF_HEIGHT))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.7, 0.8),
            ..default()
        })),
        Transform::from_xyz(0.0, -FLOOR_HALF_HEIGHT, 0.0),
    ));
}
