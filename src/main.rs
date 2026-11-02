use avian3d::prelude::*;
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use std::f32::consts::FRAC_PI_2;

const FLOOR_RADIUS: f32 = 100.0;
const FLOOR_HALF_HEIGHT: f32 = 0.5;
const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, (mouse_move, button_input).chain())
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

    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            intensity: 777_000.0,
            ..default()
        },
        Transform::from_xyz(5.0, 7.0, 5.0),
    ));
}

fn button_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut cursor_options: Single<&mut CursorOptions>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        if !cursor_options.visible {
            cursor_options.visible = true;
            cursor_options.grab_mode = CursorGrabMode::None;
        }
    }
    if mouse.any_just_pressed([MouseButton::Left, MouseButton::Right]) {
        if cursor_options.visible {
            cursor_options.visible = false;
            cursor_options.grab_mode = CursorGrabMode::Locked;
        }
    }
}

fn mouse_move(motion: Res<AccumulatedMouseMotion>, single: Single<(&Camera3d, &mut Transform)>) {
    let delta = motion.delta;
    if delta.x != 0.0 || delta.y != 0.0 {
        let delta_yaw = -delta.x * 0.003;
        let delta_pitch = -delta.y * 0.002;
        let (_camera, mut transform) = single.into_inner();
        let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
        let yaw = yaw + delta_yaw;
        let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }
}
