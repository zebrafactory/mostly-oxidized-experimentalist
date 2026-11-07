use avian3d::prelude::*;
use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow, WindowMode},
};
use std::f32::consts::FRAC_PI_2;

mod particle;
use crate::particle::*;

const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.02;
const PLAYER_RADIUS: f32 = 0.35;
const PLAYER_HALF_HEIGHT: f32 = 1.0;
const BALL_RADIUS: f32 = 1.5;
const GAP: f32 = 0.1;
const BEAM_WIDTH: f32 = BALL_RADIUS * 2.0 + GAP;
const BEAM: Vec3 = Vec3::new(ROOM.x, ROOM.y, BEAM_WIDTH - GAP);

const FLOOR_COLOR: Color = Color::srgb(0.9, 0.7, 0.8);
const WALL_COLOR: Color = Color::srgb(0.3, 0.0, 0.3);

const PLAYER_COLOR: Color = Color::srgb(0.2, 0.1, 0.1);
const BEAM_COLOR: Color = Color::srgb(0.4, 0.7, 0.9);
const BALL_COLOR: Color = Color::srgb(0.8, 0.8, 0.8);
const HEAVY_BALL_COLOR: Color = Color::srgb(0.4, 0.4, 0.8);

const LINEAR_ACCELERATION: f32 = 30.0; // m/s^2
const JUMP_IMPULSE: f32 = 17.0; // m/s

const CAMERA_3D_PERSON_DISTANCE: f32 = 4.0;

const ROOM: Vec3 = Vec3::new(20.0, 4.0, 30.0);
const WALL: f32 = 0.5;

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
struct Player {
    contacts: usize,
    in_flight: bool,
}

#[derive(Component)]
struct PlayerHead;

#[derive(Component)]
struct Ball {
    contacts: usize,
    in_flight: bool,
}

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

#[derive(Event)]
struct PlayerLanded;

#[derive(Resource, Deref)]
struct LandingSound(Handle<AudioSource>);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .add_systems(Startup, setup)
        //.add_systems(Startup, add_particle)
        .add_systems(
            Update,
            (mouse_move_yaw, mouse_move_pitch, mouse_wheel, button_input).chain(),
        )
        .add_systems(Update, (player_collision, ball_collision))
        .add_observer(ray_cast_system)
        .add_observer(reset_world)
        .add_observer(reset_balls)
        .add_observer(toggle_camera)
        .add_observer(toggle_fullscreen)
        .add_observer(un_fullscreen)
        .add_observer(play_landing_sound)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let sound = asset_server.load("breakout_collision.ogg");
    commands.insert_resource(LandingSound(sound));

    // The Player (a camera in a collider)
    commands
        .spawn((
            Player {
                contacts: 0,
                in_flight: false,
            },
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
            LinearDamping(2.5),
            GravityScale(2.5),
            Resetable::from_xyz(0.0, 0.0, 0.0),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    PlayerHead,
                    Transform::from_xyz(0.0, PLAYER_HALF_HEIGHT * 0.5, 0.0),
                    // Needed to fix:
                    // WARN bevy_app::hierarchy: warning[B0004]: Entity 549v0 with the InheritedVisibility component has a parent (548v0 entity) without InheritedVisibility.
                    InheritedVisibility::VISIBLE,
                ))
                .with_children(|sub_parent| {
                    sub_parent.spawn((
                        Camera3d::default(),
                        Transform::from_xyz(0.0, 0.0, CAMERA_3D_PERSON_DISTANCE),
                    ));
                });
        });

    // Floor and side walls
    for i in 0..8 {
        let z_offset = -(ROOM.z + BEAM_WIDTH) * i as f32;
        commands.spawn((
            RigidBody::Static,
            Collider::cuboid(ROOM.x, WALL, ROOM.z),
            Mesh3d(meshes.add(Cuboid::new(ROOM.x, WALL, ROOM.z))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: FLOOR_COLOR,
                ..default()
            })),
            Transform::from_xyz(0.0, (ROOM.y + WALL) * -0.5, z_offset),
        ));
        for sign in [-1.0, 1.0] {
            commands.spawn((
                RigidBody::Static,
                Collider::cuboid(WALL, ROOM.y, ROOM.z),
                Mesh3d(meshes.add(Cuboid::new(WALL, ROOM.y, ROOM.z))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: WALL_COLOR,
                    ..default()
                })),
                Transform::from_xyz((ROOM.x + WALL) * 0.5 * sign, 0.0, z_offset),
            ));
        }

        // The sea-saw beam
        commands.spawn((
            RigidBody::Dynamic,
            LockedAxes::ALL_LOCKED.unlock_rotation_z(),
            Collider::cuboid(BEAM.x, BEAM.y, BEAM.z),
            Mesh3d(meshes.add(Cuboid::new(BEAM.x, BEAM.y, BEAM.z))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: BEAM_COLOR,
                ..default()
            })),
            Resetable::from_xyz(0.0, 0.0, z_offset + (ROOM.z + BEAM_WIDTH) * -0.5),
        ));
        // Balls on sea-saw
        for sign in [-1.0, 1.0] {
            commands.spawn((
                Ball {
                    contacts: 0,
                    in_flight: false,
                },
                RigidBody::Dynamic,
                Collider::sphere(BALL_RADIUS),
                CollidingEntities::default(),
                GravityScale(1.0),
                Mesh3d(meshes.add(Sphere::new(BALL_RADIUS))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: BALL_COLOR,
                    ..default()
                })),
                Resetable::from_xyz(
                    sign * (ROOM.x - BEAM_WIDTH) * 0.5,
                    10.0,
                    z_offset + (ROOM.z + BEAM_WIDTH) * -0.5,
                ),
            ));
        }

        // A light
        commands.spawn((
            PointLight {
                shadow_maps_enabled: true,
                //contact_shadows_enabled: true,
                intensity: 999_000.0,
                ..default()
            },
            Transform::from_xyz(6.0, 9.0, z_offset - 6.0),
        ));
    }

    // Crosshairs for pointing your Alchemy
    commands.spawn((
        Node {
            width: Val::Px(8.0),
            height: Val::Px(8.0),
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            margin: UiRect {
                left: Val::Px(-4.0),
                top: Val::Px(-4.0),
                ..default()
            },
            ..default()
        },
        BackgroundColor(Color::WHITE),
    ));
}

fn player_collision(mut commands: Commands, single: Single<(&CollidingEntities, &mut Player)>) {
    let (colliding_entities, mut player) = single.into_inner();
    if colliding_entities.len() != player.contacts {
        match colliding_entities.len() {
            0 => {
                player.in_flight = true;
            }
            _ => {
                if player.in_flight {
                    commands.trigger(PlayerLanded);
                }
                player.in_flight = false;
            }
        }
        player.contacts = colliding_entities.len();
    }
}

fn ball_collision(mut commands: Commands, mut query: Query<(&CollidingEntities, &mut Ball)>) {
    for (colliding_entities, mut ball) in &mut query {
        if colliding_entities.len() != ball.contacts {
            match colliding_entities.len() {
                0 => {
                    ball.in_flight = true;
                }
                _ => {
                    if ball.in_flight {
                        commands.trigger(PlayerLanded);
                    }
                    ball.in_flight = false;
                }
            }
            ball.contacts = colliding_entities.len();
        }
    }
}

fn button_input(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    single: Single<(&Player, &Transform, &mut LinearVelocity)>,
    mut cursor_options: Single<&mut CursorOptions>,
    mut commands: Commands,
) {
    let (player, transform, mut linear_velocity) = single.into_inner();

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
        if !player.in_flight {
            linear_velocity.y = JUMP_IMPULSE;
        }
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

fn mouse_wheel(
    scroll: Res<AccumulatedMouseScroll>,
    mut transform: Single<&mut Transform, With<Camera3d>>,
) {
    transform.translation.z = (transform.translation.z - 1.0 * scroll.delta.y as f32)
        .clamp(0.0, CAMERA_3D_PERSON_DISTANCE);
}

fn ray_cast_system(
    _on: On<ModifyDensity>,
    mut ray_cast: MeshRayCast,
    transform: Single<&GlobalTransform, With<PlayerHead>>,
    mut query: Query<(&mut GravityScale, &MeshMaterial3d<StandardMaterial>), With<Ball>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ray = Ray3d::new(transform.translation(), transform.forward());
    let filter = |entity| query.contains(entity);
    let early_exit_test = |_entity| true;
    let settings = MeshRayCastSettings::default()
        .with_filter(&filter)
        .with_early_exit_test(&early_exit_test);
    for (entity, _hit) in ray_cast.cast_ray(ray, &settings) {
        if let Ok((mut gravity_scale, material_3d)) = query.get_mut(*entity) {
            // Changing ColliderDensity should wourk too in theory, but doesn't,
            // so use GravityScale.
            println!("hit {gravity_scale:?}");
            gravity_scale.0 = 8.0;
            if let Some(mut m) = materials.get_mut(material_3d) {
                m.base_color = HEAVY_BALL_COLOR;
            }
        }
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

fn reset_balls(
    _on: On<ResetWorld>,
    mut query: Query<(&mut GravityScale, &MeshMaterial3d<StandardMaterial>), With<Ball>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("reset balls");
    for (mut gravity_scale, material_3d) in &mut query {
        gravity_scale.0 = 1.0;
        if let Some(mut m) = materials.get_mut(material_3d) {
            m.base_color = BALL_COLOR;
        }
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

fn play_landing_sound(_on: On<PlayerLanded>, mut commands: Commands, sound: Res<LandingSound>) {
    commands.spawn((AudioPlayer(sound.clone()), PlaybackSettings::DESPAWN));
}
