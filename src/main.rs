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
const PLAYER_RADIUS: f32 = 0.4;
const PLAYER_HALF_HEIGHT: f32 = 1.0;
const BALL_RADIUS: f32 = 0.7;
const Z_OFFSET: f32 = -6.0;
const BEAM_LENGTH: f32 = 6.0;
const BEAM_HEIGHT: f32 = 0.25;
const BEAM_WIDTH: f32 = BALL_RADIUS * 2.0;

const FLOOR_COLOR: Color = Color::srgb(0.9, 0.7, 0.8);
const PLAYER_COLOR: Color = Color::srgb(0.2, 0.1, 0.1);
const BEAM_COLOR: Color = Color::srgb(0.4, 0.7, 0.9);
const BALL_COLOR: Color = Color::srgb(0.8, 0.8, 0.8);
const CENTER_BALL_COLOR: Color = Color::srgb(0.3, 0.0, 0.3);

const LINEAR_ACCELERATION: f32 = 30.0; // m/s^2
const JUMP_IMPULSE: f32 = 11.0; // m/s

const CAMERA_3D_PERSON_DISTANCE: f32 = 3.5;

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
struct Player;

#[derive(Component)]
struct PlayerHead;

#[derive(Component)]
struct Ball;

#[derive(Event)]
struct ResetWorld;

#[derive(Event)]
struct ModifyDensity;

#[derive(Event)]
struct ToggleCamera;

#[derive(Event)]
struct ToggleFullscreen;

#[derive(Event)]
struct UnFullscreen;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .add_systems(Startup, setup)
        .add_systems(Startup, add_particle)
        .add_systems(
            Update,
            (mouse_move_yaw, mouse_move_pitch, button_input).chain(),
        )
        .add_systems(Update, player_collision)
        .add_observer(ray_cast_system)
        .add_observer(reset_world)
        .add_observer(toggle_camera)
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
    commands
        .spawn((
            Player,
            RigidBody::Dynamic,
            Collider::capsule(PLAYER_RADIUS, PLAYER_HALF_HEIGHT),
            CollidingEntities::default(),
            Mesh3d(meshes.add(Capsule3d::new(PLAYER_RADIUS, PLAYER_HALF_HEIGHT))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: PLAYER_COLOR,
                alpha_mode: AlphaMode::Add,
                ..default()
            })),
            LockedAxes::ROTATION_LOCKED,
            LinearDamping(2.0),
            GravityScale(2.0),
            Resetable::from_xyz(0.0, PLAYER_RADIUS * 8.0, 0.0),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    PlayerHead,
                    Transform::from_xyz(0.0, PLAYER_HALF_HEIGHT * 0.5, 0.0),
                ))
                .with_children(|sub_parent| {
                    sub_parent.spawn((
                        Camera3d::default(),
                        Transform::from_xyz(0.0, 0.0, CAMERA_3D_PERSON_DISTANCE),
                    ));
                });
        });

    // Static floor
    commands.spawn((
        RigidBody::Static,
        Collider::cylinder(FLOOR_RADIUS, FLOOR_HALF_HEIGHT),
        Mesh3d(meshes.add(Cylinder::new(FLOOR_RADIUS, FLOOR_HALF_HEIGHT))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: FLOOR_COLOR,
            ..default()
        })),
        Transform::from_xyz(0.0, -FLOOR_HALF_HEIGHT * 0.5, 0.0),
    ));

    for i in 0..4 {
        let y_offset = 2.0 * i as f32;
        let z_offset = Z_OFFSET * (i + 1) as f32;

        // Static sphere the beam balances on
        commands.spawn((
            RigidBody::Static,
            Collider::sphere(BALL_RADIUS),
            Mesh3d(meshes.add(Sphere::new(BALL_RADIUS))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: CENTER_BALL_COLOR,
                ..default()
            })),
            Resetable::from_xyz(0.0, y_offset + BALL_RADIUS, z_offset),
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
            Resetable::from_xyz(
                0.0,
                y_offset + BALL_RADIUS * 2.0 + BEAM_HEIGHT * 0.5,
                z_offset,
            ),
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
                    y_offset + BALL_RADIUS * 3.0 + BEAM_HEIGHT,
                    z_offset,
                ),
            ));
        }
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

fn player_collision(colliding_entities: Single<&CollidingEntities, With<Player>>) {
    println!("collision with {}", colliding_entities.len());
}

fn button_input(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    player: Single<(&Transform, &mut LinearVelocity), With<Player>>,
    mut cursor_options: Single<&mut CursorOptions>,
    mut commands: Commands,
) {
    let (transform, mut linear_velocity) = player.into_inner();

    let mut input = Vec2::ZERO;
    if keyboard.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp, KeyCode::KeyO]) {
        // Forward
        input.y -= 1.0;
    }
    if keyboard.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown, KeyCode::KeyL]) {
        // Backward
        input.y += 1.0;
    }
    if keyboard.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft, KeyCode::KeyK]) {
        // Strafe Left
        input.x -= 1.0;
    }
    if keyboard.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight, KeyCode::Semicolon]) {
        // Strafe Right
        input.x += 1.0;
    }
    if keyboard.just_pressed(KeyCode::Space) {
        linear_velocity.y = JUMP_IMPULSE;
    }
    if input != Vec2::ZERO {
        let delta = input.normalize() * LINEAR_ACCELERATION * time.delta_secs();
        let delta = (transform.local_x() * delta.x) + (transform.local_z() * delta.y);
        linear_velocity.0 += Vec3::new(delta.x, 0.0, delta.z);
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
    if keyboard.just_pressed(KeyCode::F5) {
        commands.trigger(ToggleCamera)
    }
    if keyboard.just_pressed(KeyCode::F11) {
        commands.trigger(ToggleFullscreen)
    }
}

fn mouse_move_yaw(
    motion: Res<AccumulatedMouseMotion>,
    cursor_options: Single<&mut CursorOptions>,
    mut transform: Single<&mut Transform, With<Player>>,
) {
    if !cursor_options.visible && cursor_options.grab_mode == CursorGrabMode::Locked {
        let delta = motion.delta;
        if delta.x != 0.0 {
            let delta_yaw = -delta.x * 0.003;
            let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
            let yaw = yaw + delta_yaw;
            transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
        }
    }
}

fn mouse_move_pitch(
    motion: Res<AccumulatedMouseMotion>,
    cursor_options: Single<&mut CursorOptions>,
    mut transform: Single<&mut Transform, With<PlayerHead>>,
) {
    if !cursor_options.visible && cursor_options.grab_mode == CursorGrabMode::Locked {
        let delta = motion.delta;
        if delta.y != 0.0 {
            let delta_pitch = -delta.y * 0.002;
            let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
            let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);
            transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
        }
    }
}

fn ray_cast_system(
    _on: On<ModifyDensity>,
    mut ray_cast: MeshRayCast,
    transform: Single<&GlobalTransform, With<PlayerHead>>,
    query: Query<&MeshMaterial3d<StandardMaterial>, With<Ball>>,
    mut commands: Commands,
) {
    let ray = Ray3d::new(transform.translation(), transform.forward());
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

fn toggle_camera(_on: On<ToggleCamera>, mut transform: Single<&mut Transform, With<Camera3d>>) {
    println!("toggle camera");
    transform.translation.z = match transform.translation.z {
        0.0 => CAMERA_3D_PERSON_DISTANCE,
        _ => 0.0,
    };
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
