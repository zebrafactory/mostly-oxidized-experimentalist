use avian3d::prelude::*;
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow, WindowMode},
};
use std::f32::consts::FRAC_PI_2;

mod particle;
use crate::particle::*;

const FLOOR_RADIUS: f32 = 100.0;
const FLOOR_HALF_HEIGHT: f32 = 0.5;
const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.02;
const PLAYER_RADIUS: f32 = 0.5;
const BALL_RADIUS: f32 = 0.7;
const Z_OFFSET: f32 = -6.0;
const BEAM_LENGTH: f32 = 6.0;
const BEAM_HEIGHT: f32 = 0.25;
const BEAM_WIDTH: f32 = BALL_RADIUS * 2.0;

const FLOOR_COLOR: Color = Color::srgb(0.9, 0.7, 0.8);
const PLAYER_COLOR: Color = Color::srgb(0.1, 0.1, 0.1);
const BEAM_COLOR: Color = Color::srgb(0.4, 0.7, 0.9);
const BALL_COLOR: Color = Color::srgb(0.8, 0.8, 0.8);
const CENTER_BALL_COLOR: Color = Color::srgb(0.3, 0.0, 0.3);

const LINEAR_ACCELERATION: f32 = 30.0; // m/s^2
const JUMP_IMPULSE: f32 = 11.0; // m/s

#[derive(Component)]
struct Resetable {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

impl Resetable {
    fn from_xyz(x: f32, y: f32, z: f32) -> (Self, Transform) {
        let transform = Transform::from_xyz(x, y, z);
        Self::from_transform(transform)
    }

    fn from_transform(transform: Transform) -> (Self, Transform) {
        (
            Resetable {
                translation: transform.translation,
                rotation: transform.rotation,
                scale: transform.scale,
            },
            transform,
        )
    }

    fn reset(&self, transform: &mut Transform) {
        transform.translation = self.translation;
        transform.rotation = self.rotation;
        transform.scale = self.scale;
    }
}

#[derive(Component)]
struct Ball;

#[derive(Event)]
struct ResetWorld;

#[derive(Event)]
struct ModifyDensity;

#[derive(Event)]
struct ToggleFullscreen;

#[derive(Event)]
struct UnFullscreen;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .add_systems(Startup, setup)
        .add_systems(Startup, add_particle)
        .add_systems(Update, (mouse_move, button_input).chain())
        .add_observer(ray_cast_system)
        .add_observer(reset_world)
        .add_observer(toggle_fullscreen)
        .add_observer(un_fullscreen)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // The Player (a camera in a collider)
    commands.spawn((
        RigidBody::Dynamic,
        LockedAxes::ROTATION_LOCKED,
        Collider::sphere(PLAYER_RADIUS),
        Mesh3d(meshes.add(Sphere::new(PLAYER_RADIUS))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: PLAYER_COLOR,
            ..default()
        })),
        LinearDamping(2.0),
        GravityScale(2.0),
        Camera3d::default(),
        Resetable::from_xyz(0.0, PLAYER_RADIUS * 8.0, 0.0),
    ));

    // Static floor
    commands.spawn((
        RigidBody::Static,
        Collider::cylinder(FLOOR_RADIUS, FLOOR_HALF_HEIGHT),
        Mesh3d(meshes.add(Cylinder::new(FLOOR_RADIUS, FLOOR_HALF_HEIGHT))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: FLOOR_COLOR,
            ..default()
        })),
        Transform::from_xyz(0.0, -FLOOR_HALF_HEIGHT / 2.0, 0.0),
    ));

    // Static sphere the beam balances on
    commands.spawn((
        RigidBody::Static,
        Collider::sphere(BALL_RADIUS),
        Mesh3d(meshes.add(Sphere::new(BALL_RADIUS))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: CENTER_BALL_COLOR,
            ..default()
        })),
        Resetable::from_xyz(0.0, BALL_RADIUS, Z_OFFSET),
    ));

    // The balance beam
    commands.spawn((
        RigidBody::Dynamic,
        Collider::cuboid(BEAM_LENGTH, BEAM_HEIGHT, BEAM_WIDTH),
        Mesh3d(meshes.add(Cuboid::new(BEAM_LENGTH, BEAM_HEIGHT, BEAM_WIDTH))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: BEAM_COLOR,
            ..default()
        })),
        Resetable::from_xyz(0.0, BALL_RADIUS * 2.0 + BEAM_HEIGHT * 0.5, Z_OFFSET),
    ));

    // The dynamic spheres at each end of the balance beam
    for sign in [-0.45, 0.45] {
        commands.spawn((
            Ball,
            RigidBody::Dynamic,
            Collider::sphere(BALL_RADIUS),
            Mesh3d(meshes.add(Sphere::new(BALL_RADIUS))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: BALL_COLOR,
                ..default()
            })),
            Resetable::from_xyz(
                sign * BEAM_LENGTH,
                BALL_RADIUS * 3.0 + BEAM_HEIGHT,
                Z_OFFSET,
            ),
        ));
    }

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
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    player: Single<(&Transform, &mut LinearVelocity), With<Camera3d>>,
    mut cursor_options: Single<&mut CursorOptions>,
    mut commands: Commands,
) {
    let mut input = Vec3::ZERO;
    let (transform, mut linear_velocity) = player.into_inner();

    if keyboard.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        // Forward
        input.z -= 1.0;
    }
    if keyboard.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        // Backward
        input.z += 1.0;
    }
    if keyboard.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        // Strafe Left
        input.x -= 1.0;
    }
    if keyboard.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        // Strafe Right
        input.x += 1.0;
    }
    if keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        // Fly Down
        input.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::Space) {
        // Fly Up
        input.y += 1.0;
    }
    if keyboard.just_pressed(KeyCode::Space) {
        linear_velocity.y = JUMP_IMPULSE;
    }

    if input != Vec3::ZERO {
        let delta = input * LINEAR_ACCELERATION * time.delta_secs();
        let local_x = Vec2::new(transform.local_x().x, transform.local_x().z).normalize();
        let local_z = Vec2::new(transform.local_z().x, transform.local_z().z).normalize();
        let delta = (local_x * delta.x) + (local_z * delta.z);
        linear_velocity.0 += Vec3::new(delta.x, 0.0, delta.y);
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        cursor_options.visible = true;
        cursor_options.grab_mode = CursorGrabMode::None;
        commands.trigger(UnFullscreen);
    }
    if mouse.any_just_pressed([MouseButton::Left, MouseButton::Right]) {
        if cursor_options.visible || cursor_options.grab_mode != CursorGrabMode::Locked {
            cursor_options.visible = false;
            cursor_options.grab_mode = CursorGrabMode::Locked;
        } else {
            commands.trigger(ModifyDensity);
        }
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        commands.trigger(ResetWorld);
    }
    if keyboard.just_pressed(KeyCode::F11) {
        commands.trigger(ToggleFullscreen)
    }
}

fn mouse_move(
    motion: Res<AccumulatedMouseMotion>,
    cursor_options: Single<&mut CursorOptions>,
    mut transform: Single<&mut Transform, With<Camera3d>>,
) {
    if !cursor_options.visible && cursor_options.grab_mode == CursorGrabMode::Locked {
        let delta = motion.delta;
        if delta.x != 0.0 || delta.y != 0.0 {
            let delta_yaw = -delta.x * 0.003;
            let delta_pitch = -delta.y * 0.002;
            let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
            let yaw = yaw + delta_yaw;
            let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);
            transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
        }
    }
}

fn ray_cast_system(
    _on: On<ModifyDensity>,
    mut ray_cast: MeshRayCast,
    transform: Single<&Transform, With<Camera3d>>,
    query: Query<&MeshMaterial3d<StandardMaterial>, With<Ball>>,
    mut commands: Commands,
) {
    let ray = Ray3d::new(transform.translation, transform.forward());
    let filter = |entity| query.contains(entity);
    let early_exit_test = |_entity| false;
    let settings = MeshRayCastSettings::default()
        .with_filter(&filter)
        .with_early_exit_test(&early_exit_test);
    for (entity, _hit) in ray_cast.cast_ray(ray, &settings) {
        commands.entity(*entity).despawn();
    }
}

fn reset_world(
    _on: On<ResetWorld>,
    mut query: Query<(
        &Resetable,
        &mut Transform,
        &mut LinearVelocity,
        &mut AngularVelocity,
    )>,
) {
    for (resetable, mut transform, mut linear_velocity, mut angular_velocity) in &mut query {
        resetable.reset(&mut transform);
        linear_velocity.0 = Vec3::ZERO;
        angular_velocity.0 = Vec3::ZERO;
    }
}

fn toggle_fullscreen(
    _on: On<ToggleFullscreen>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut cursor_options: Single<&mut CursorOptions>,
) {
    window.mode = match window.mode {
        WindowMode::Windowed => {
            cursor_options.visible = false;
            cursor_options.grab_mode = CursorGrabMode::Locked;
            WindowMode::BorderlessFullscreen(MonitorSelection::Primary)
        }
        _ => {
            cursor_options.visible = true;
            cursor_options.grab_mode = CursorGrabMode::None;
            WindowMode::Windowed
        }
    };
}

fn un_fullscreen(_on: On<UnFullscreen>, mut window: Single<&mut Window, With<PrimaryWindow>>) {
    window.mode = WindowMode::Windowed;
}
