#![doc = include_str!("../README.md")]

use bevy::{
    input::mouse::MouseWheel,
    prelude::*,
};

/// A `Plugin` providing the systems and components required to make a ScrollView work.
///
/// # Example
/// ```
/// use bevy::prelude::*;
/// use bevy_simple_scroll_view::*;
///
/// App::new()
///     .add_plugins((DefaultPlugins,ScrollViewPlugin))
///     .run();
/// ```
pub struct ScrollViewPlugin;

impl Plugin for ScrollViewPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ScrollView>()
            .register_type::<ScrollableContent>()
            .add_systems(
                Update,
                (
                    create_scroll_view,
                    input_mouse_pressed_move,
                    input_touch_pressed_move,
                    scroll_events,
                    fling_update,
                    scroll_update,
                )
                    .chain(),
            );
    }
}

/// Root component of scroll, it should have clipped style.
#[derive(Component, Debug, Reflect)]
pub struct ScrollView {
    /// Field which control speed of the scrolling.
    /// Could be negative number to implement invert scroll
    pub scroll_speed: f32,
    pub friction: f32,
    pub old_mouse_y: Option<f32>,
    pub velocity: f32,
    pub max_scroll: f32,
}

impl Default for ScrollView {
    fn default() -> Self {
        Self {
            scroll_speed: 200.0,
            friction: 4.2,
            old_mouse_y: None,
            velocity: 0.0,
            max_scroll: 0.0,
        }
    }
}

/// Component containing offset value of the scroll container to the parent.
/// It is possible to update the field `pos_y` manually to move scrollview to desired location.
#[derive(Component, Debug, Reflect, Default)]
pub struct ScrollableContent {
    /// Scroll container offset to the `ScrollView`.
    pub pos_y: f32,
}

pub fn create_scroll_view(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Style), Added<ScrollView>>,
) {
    for (e, mut style) in q.iter_mut() {
        style.overflow = Overflow::clip();
        style.align_items = AlignItems::Start;
        style.align_self = AlignSelf::Stretch;
        style.flex_direction = FlexDirection::Row;
        commands.entity(e).insert(Interaction::None);
    }
}

fn input_mouse_pressed_move(
    mut q: Query<(&Children, &Interaction, &Node, &mut ScrollView)>,
    mut content_q: Query<(&mut ScrollableContent, &Node)>,
    windows: Query<&mut Window>,
    time: Res<Time>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    if let Some(pos) = window.cursor_position() {
        for (children, &interaction, node, mut view) in q.iter_mut() {
            if interaction != Interaction::Pressed {
                view.old_mouse_y = None;
                continue;
            }
            let delta = if let Some(old_y) = view.old_mouse_y {
                pos.y - old_y
            } else {
                0.0
            };
            view.old_mouse_y = Some(pos.y);
            view.velocity = (view.velocity + delta / time.delta_seconds()) / 2.0;
            view.max_scroll = 0.0;
            // iterate children and find the bottom of the last one
            for &child in children.iter() {
                if let Ok(item) = content_q.get_mut(child) {
                    view.max_scroll = view.max_scroll.min(-(item.1.size().y - node.size().y).max(0.0));
                }
            }
        }
    }
}

fn input_touch_pressed_move(
    touches: Res<Touches>,
    mut q: Query<(&Children, &Interaction, &Node, &mut ScrollView)>,
    mut content_q: Query<(&mut ScrollableContent, &Node)>,
    time: Res<Time>,
) {
    for t in touches.iter() {
        let Some(touch) = touches.get_pressed(t.id()) else {
            continue;
        };

        for (children, &interaction, node, mut view) in q.iter_mut() {
            if interaction != Interaction::Pressed {
                continue;
            }
            view.velocity = (view.velocity + touch.delta().y / time.delta_seconds()) / 2.0;
            view.max_scroll = 0.0;
            for &child in children.iter() {
                if let Ok(item) = content_q.get_mut(child) {
                    view.max_scroll = view.max_scroll.min(-(item.1.size().y - node.size().y).max(0.0));
                }
            }
        }
    }
}

fn scroll_events(
    mut scroll_evr: EventReader<MouseWheel>,
    mut q: Query<(&Children, &Interaction, &ScrollView, &Node), With<ScrollView>>,
    time: Res<Time>,
    mut content_q: Query<(&mut ScrollableContent, &Node)>,
) {
    use bevy::input::mouse::MouseScrollUnit;
    for ev in scroll_evr.read() {
        for (children, &interaction, scroll_view, node) in q.iter_mut() {
            let y = match ev.unit {
                MouseScrollUnit::Line => {
                    ev.y * time.delta().as_secs_f32() * scroll_view.scroll_speed
                }
                MouseScrollUnit::Pixel => ev.y,
            };
            if interaction != Interaction::Hovered {
                continue;
            }
            let container_height = node.size().y;

            for &child in children.iter() {
                if let Ok(item) = content_q.get_mut(child) {
                    let y = y * time.delta().as_secs_f32() * scroll_view.scroll_speed;
                    let mut scroll = item.0;
                    let max_scroll = (item.1.size().y - container_height).max(0.0);
                    scroll.pos_y += y;
                    scroll.pos_y = scroll.pos_y.clamp(-max_scroll, 0.);
                }
            }
        }
    }
}

fn scroll_update(mut q: Query<(&ScrollableContent, &mut Style), Changed<ScrollableContent>>) {
    for (scroll, mut style) in q.iter_mut() {
        style.top = Val::Px(scroll.pos_y);
    }
}

fn fling_update(
    mut q_view: Query<(&mut ScrollView, &Children)>,
    mut q_scroll: Query<&mut ScrollableContent>,
    time: Res<Time>,
) {
    for (mut view, children) in q_view.iter_mut() {
        let mut iter = q_scroll.iter_many_mut(children);
        while let Some(mut scroll) = iter.fetch_next() {
            if view.velocity.abs() > 16.0 {
                let (value, velocity) = calc_velocity(scroll.pos_y, view.velocity, -view.friction, time.delta_seconds());
                view.velocity = velocity;
                scroll.pos_y = value;
                scroll.pos_y = scroll.pos_y.clamp(view.max_scroll, 0.);
            } else {
                view.velocity = 0.0;
            }
        }
    }
}

fn calc_velocity(value: f32, velocity: f32, friction: f32, delta_t: f32) -> (f32, f32) {
    (
        value - velocity / friction + velocity / friction * (friction * delta_t).exp(),
        velocity * (delta_t * friction).exp(),
    )
}

