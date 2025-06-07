# From Zero to Smooth: Crafting Motion Algorithms That Respect Physical Limits

While trying to animate a snake I've been thinking about how to get natural and smooth motions. In order to get a smooth trajectory you need some kind of constrained motion planning that tells your position over time. This has led me down a rabbit hole in motion planning for robotic systems.

In robotics, joints need to change a value from a to b, this value usually means the amount of power that goes to an actuator. So essentially the value gets translated into a position you want to be in. What this article is about is how change this value in order to minimize the stress on a joint. This is done by trying to make changes in velocity, acceleration and jerk continuous and smooth but also considering the bounds of a system. All mechanical/biological systems have bounds on how fast they can move. That means that you want to consider bounds on velocity, acceleration and jerk and possibly higher derivates. So this is something we have to consider as well which makes solutions less obvious and tedious. This is because you have to consider the time intervals in which direction you change values.



# References
- [S-curve 3,4,5th order polynomial vs trigonometric](./from_zero_to_smooth/Kinematically%20Constrained%20Jerk–Continuous%20S-Curve%20Trajectory_Planning%20in%20Joint%20Space%20for%20Industrial%20Robots.pdf)
- [Continuous jerk s-curve algorithm](./from_zero_to_smooth/Kinematically%20Constrained%20Jerk–Continuous%20S-Curve%20Trajectory_Planning%20in%20Joint%20Space%20for%20Industrial%20Robots.pdf)
- [7th degree b-spline curve interpolation method](./from_zero_to_smooth/nguyen-et-al-2008-on-algorithms-for-planning-s-curve-motion-profiles.pdf)
