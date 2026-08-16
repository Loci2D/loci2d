// Pure deterministic navigation, waypoint tracking, and destination steering (ADR-0013, Milestone 5.4).
// Guarantees 100% bit-exact cross-platform determinism via fixed-point arithmetic (I16F16).

use super::math::deterministic_distance;
use crate::world::fixed_point::DeterministicVector2;
use fixed::types::I16F16;

/// Server-side navigation state for destination steering and waypoint traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationComponent {
    /// Active destination target in fixed-point world coordinates.
    pub target: Option<DeterministicVector2>,
    /// Arrival tolerance distance threshold (recommended >= move_speed) to stop without jitter.
    pub arrival_tolerance: I16F16,
    /// Movement speed magnitude per tick.
    pub move_speed: I16F16,
    /// Ordered list of sequential waypoints.
    pub waypoints: Vec<DeterministicVector2>,
    /// Current target waypoint index in `waypoints`.
    pub current_waypoint_index: usize,
}

impl NavigationComponent {
    /// Creates a new navigation component with specified speed and arrival tolerance.
    pub fn new(move_speed: I16F16, arrival_tolerance: I16F16) -> Self {
        Self {
            target: None,
            arrival_tolerance,
            move_speed,
            waypoints: Vec::new(),
            current_waypoint_index: 0,
        }
    }

    /// Sets an immediate destination target, clearing any queued waypoints.
    pub fn set_target(&mut self, target: DeterministicVector2) {
        self.target = Some(target);
        self.waypoints.clear();
        self.current_waypoint_index = 0;
    }

    /// Sets a sequence of waypoints. If non-empty, sets the initial target to waypoints[0].
    pub fn set_waypoints(&mut self, waypoints: Vec<DeterministicVector2>) {
        if waypoints.is_empty() {
            self.clear();
        } else {
            self.target = Some(waypoints[0]);
            self.waypoints = waypoints;
            self.current_waypoint_index = 0;
        }
    }

    /// Appends a waypoint to the waypoint queue. If there is currently no active target, activates this waypoint immediately.
    pub fn push_waypoint(&mut self, waypoint: DeterministicVector2) {
        if self.target.is_none() && self.waypoints.is_empty() {
            self.target = Some(waypoint);
            self.waypoints.push(waypoint);
            self.current_waypoint_index = 0;
        } else {
            self.waypoints.push(waypoint);
        }
    }

    /// Clears active navigation target and all waypoints.
    pub fn clear(&mut self) {
        self.target = None;
        self.waypoints.clear();
        self.current_waypoint_index = 0;
    }

    /// Returns true if currently steering towards a target.
    pub fn is_navigating(&self) -> bool {
        self.target.is_some()
    }
}

/// Evaluates destination steering for an entity on a fixed tick (ADR-0013).
/// Updates `velocity` toward the target or advances through waypoints.
pub fn update_entity_navigation(
    position: DeterministicVector2,
    velocity: &mut DeterministicVector2,
    nav: &mut NavigationComponent,
) {
    let Some(target) = nav.target else {
        return;
    };

    let dist = deterministic_distance(position, target);

    // If already at target (zero distance) or within arrival tolerance (when not stepping into it):
    if dist == I16F16::ZERO || (dist <= nav.arrival_tolerance && dist > nav.move_speed) {
        if nav.current_waypoint_index + 1 < nav.waypoints.len() {
            nav.current_waypoint_index += 1;
            let next_target = nav.waypoints[nav.current_waypoint_index];
            nav.target = Some(next_target);

            let next_dist = deterministic_distance(position, next_target);
            if next_dist > I16F16::ZERO {
                if next_dist <= nav.move_speed {
                    *velocity = next_target - position;
                } else {
                    let dir = (next_target - position) / next_dist;
                    *velocity = dir * nav.move_speed;
                }
            } else {
                *velocity = DeterministicVector2::ZERO;
            }
        } else {
            // Reached final destination — cleanly stop and clear navigation state
            *velocity = DeterministicVector2::ZERO;
            nav.target = None;
            nav.waypoints.clear();
            nav.current_waypoint_index = 0;
        }
    } else {
        // Steer toward target: if within 1 tick step, clamp displacement to reach target exactly
        if dist <= nav.move_speed {
            *velocity = target - position;
        } else {
            let dir = (target - position) / dist;
            *velocity = dir * nav.move_speed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_component_lifecycle() {
        let mut nav = NavigationComponent::new(I16F16::from_num(2), I16F16::from_num(1));
        assert!(!nav.is_navigating());
        assert_eq!(nav.target, None);

        nav.set_target(DeterministicVector2::new(
            I16F16::from_num(10),
            I16F16::from_num(20),
        ));
        assert!(nav.is_navigating());
        assert_eq!(
            nav.target,
            Some(DeterministicVector2::new(
                I16F16::from_num(10),
                I16F16::from_num(20)
            ))
        );

        nav.clear();
        assert!(!nav.is_navigating());
        assert_eq!(nav.target, None);
    }

    #[test]
    fn test_navigation_waypoints_queue() {
        let mut nav = NavigationComponent::new(I16F16::from_num(1), I16F16::from_num(1));
        let wp1 = DeterministicVector2::new(I16F16::from_num(5), I16F16::ZERO);
        let wp2 = DeterministicVector2::new(I16F16::from_num(5), I16F16::from_num(5));
        let wp3 = DeterministicVector2::new(I16F16::ZERO, I16F16::from_num(5));

        nav.set_waypoints(vec![wp1, wp2, wp3]);
        assert_eq!(nav.target, Some(wp1));
        assert_eq!(nav.waypoints.len(), 3);
        assert_eq!(nav.current_waypoint_index, 0);
    }
}
