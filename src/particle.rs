use avian3d::prelude::*;
use bevy::{
    prelude::*,
};


#[derive(Component)]
pub struct Particle;

pub fn add_particle(mut commands: Commands, 
                    mut meshes: ResMut<Assets<Mesh>>,
                    mut materials: ResMut<Assets<StandardMaterial>>,) {
    commands.spawn((Particle, 
        RigidBody::Static,
        Collider::sphere(1.8),
        Mesh3d(meshes.add(Sphere::new(1.8))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.1, 0.8),
            ..default()
        })),
        ));

}

