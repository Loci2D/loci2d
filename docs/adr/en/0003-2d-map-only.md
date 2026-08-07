# ADR 0003: 2D Map Only Architecture
 
## Status
 
Accepted
 
## Context
 
Full 3D game servers introduce significant complexity:
- **3D Spatial Calculations**: Complex collision detection, raycasting, volumetric calculations
- **Navigation**: 3D pathfinding is computationally expensive and difficult to implement
- **Physics**: 3D physics engines add substantial complexity and performance overhead
- **Network Bandwidth**: 3D positions and rotations require more data to transmit
- **Learning Curve**: 3D math (quaternions, matrices) is harder for beginners
 
Many successful games use 2D gameplay even with 3D visuals (MOBAs, strategy games, isometric RPGs).
 
## Decision
 
We choose to support **2D maps only** where:
- All game logic operates on a 2D plane (X, Y coordinates)
- Vector2 is the primary position type throughout the codebase
- Games can still have 3D visuals/rendering on the client side
- Server never deals with Z-axis, height, or 3D rotations
- Simplified collision detection using 2D bounding boxes/circles
- 2D pathfinding algorithms (A*, Dijkstra) are sufficient
 
This supports game types like:
- **MOBA games**: 3D characters on 2D map
- **Strategy games**: Top-down or isometric views
- **2D platformers**: Side-scrolling gameplay
- **Isometric RPGs**: 2D logic with 3D presentation
 
## Consequences
 
**Positive:**
- **Reduced Complexity**: 2D math is simpler and easier to debug
- **Better Performance**: 2D calculations are faster than 3D
- **Lower Bandwidth**: 2D positions require less network data
- **Easier Learning**: Beginners can focus on game logic instead of 3D math
- **Simpler Pathfinding**: 2D pathfinding is well-understood and efficient
- **Faster Development**: Less time spent on 3D spatial problems
- **Genre Coverage**: Still supports many popular game genres
 
**Negative:**
- **Limited Gameplay**: Can't support true 3D gameplay (flight, underwater, multi-level)
- **Visual Restrictions**: 3D visuals must conform to 2D gameplay constraints
- **Height Limitations**: No true vertical gameplay elements
- **Camera Constraints**: Camera angles limited by 2D gameplay
