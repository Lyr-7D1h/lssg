# Motion Planning

Trying to animate a snake I've been thinking about how to get natural and smooth motions. In order to get a smooth trajectory you need some kind of constrained motion planning that tells your position over time. This has led me down a rabbit hole in motion planning for robotic systems.

In robotics, joints need to change a value from a to b, this gets translated into a position you want to be in. What this article is about is how this translation is done. In order to minimize the stress on a joint you want to try to make changes in acceleration continuous and smooth, you can do this by making jerk continuous.
