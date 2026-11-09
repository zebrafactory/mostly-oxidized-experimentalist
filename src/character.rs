use avian3d::prelude::*;
use bevy::{prelude::*, world_serialization::WorldInstanceReady};

#[derive(Component)]
pub struct Character;

#[derive(Component)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}

#[derive(Clone, Resource)]
pub struct CharacterAnimationGraph(Handle<AnimationGraph>);

#[derive(Component)]
pub struct CharacterAnimationWeights {
    weights: [f32; 3],
}

static CLIP_NODE_INDICES: [u32; 3] = [2, 3, 4];

pub fn add_character(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let (graph, index) = AnimationGraph::from_clip(
        asset_server.load(GltfAssetLabel::Animation(2).from_asset("homunculus.glb")),
    );

    let graph_handle = graphs.add(graph);

    let mut animation_graph = AnimationGraph::new();
    let blend_node = animation_graph.add_blend(1.0, animation_graph.root);
    animation_graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(1).from_asset("homunculus.glb")),
        2.0,
        animation_graph.root,
    );
    animation_graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(2).from_asset("homunculus.glb")),
        1.0,
        animation_graph.root,
    );
    animation_graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(0).from_asset("homunculus.glb")),
        3.0,
        animation_graph.root,
    );

    let handle = graphs.add(animation_graph);
    commands.insert_resource(CharacterAnimationGraph(handle));

    let mesh =
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("homunculus.glb")));

    commands.spawn((Character, mesh, Transform::from_xyz(0.0, -2.5, 0.0)));
}

pub fn init_character_animations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut AnimationPlayer)>,
    animation_graph: Res<CharacterAnimationGraph>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }

    for (entity, mut player) in query.iter_mut() {
        commands.entity(entity).insert((
            AnimationGraphHandle(animation_graph.0.clone()),
            CharacterAnimationWeights::default(),
        ));
        for &node_index in &CLIP_NODE_INDICES {
            player.play(node_index.into()).repeat();
        }

        *done = true;
    }
}

pub fn sync_character_weights(
    mut query: Query<(&mut AnimationPlayer, &CharacterAnimationWeights)>,
) {
    for (mut animation_player, animation_weights) in query.iter_mut() {
        for (&animation_node_index, &animation_weight) in CLIP_NODE_INDICES
            .iter()
            .zip(animation_weights.weights.iter())
        {
            if !animation_player.is_playing_animation(animation_node_index.into()) {
                animation_player.play(animation_node_index.into());
            }

            if let Some(active_animation) =
                animation_player.animation_mut(animation_node_index.into())
            {
                active_animation.set_weight(animation_weight);
            }
        }
    }
}

impl Default for CharacterAnimationWeights {
    fn default() -> Self {
        Self { weights: [1.0; 3] }
    }
}
