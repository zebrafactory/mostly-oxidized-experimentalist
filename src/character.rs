use avian3d::prelude::*;
use bevy::{prelude::*, world_serialization::WorldInstanceReady};

#[derive(Component)]
pub struct Character;

#[derive(Component)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}

pub fn add_character(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let (graph, index) = AnimationGraph::from_clip(
        asset_server.load(GltfAssetLabel::Animation(0).from_asset("homunculus.glb")),
    );

    let graph_handle = graphs.add(graph);

    let animation_to_play = AnimationToPlay {
        graph_handle,
        index,
    };

    let mesh =
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("homunculus.glb")));

    commands
        .spawn((animation_to_play, mesh))
        .observe(play_animation_when_ready);
}

fn play_animation_when_ready(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations_to_play: Query<&AnimationToPlay>,
    mut players: Query<&mut AnimationPlayer>,
) {
    if let Ok(animation_to_play) = animations_to_play.get(scene_ready.entity) {
        for child in children.iter_descendants(scene_ready.entity) {
            if let Ok(mut player) = players.get_mut(child) {
                player.play(animation_to_play.index).repeat();
                commands
                    .entity(child)
                    .insert(AnimationGraphHandle(animation_to_play.graph_handle.clone()));
            }
        }
    }
}
