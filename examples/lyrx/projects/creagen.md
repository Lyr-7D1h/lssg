<!--
[post]
created_on="2025-6-14"
render = false
-->
# Creagen: Creative Coding Editor

Creagen stands for Creative Generations. It is a creative coding web editor. Its goal is to provide easy accessibility to make creative coding projects. Its design goals are to be **visually minimalistic**, **powerful**, **fast** and **easy to use**. 

See the [up-to-date documentation](https://creagen.dev/docs) for more information.


## Example: Particle Vector Field 

1. Select the [creagen](https://www.npmjs.com/package/creagen) package with version `0.0.12` in settings

2. Copy and paste 
```ts
// Particle Vector field
import { Canvas, Vector, load, vec, draw, Random, Color } from "creagen";

const SPEED = 0.1;
const CENTER_POINT = vec(0, 0);
const WIDTH = 10;

const POINTS = 10000;

const c = Canvas.create();
const height = WIDTH * (c.height / c.width);
const scale = c.width / 2 / WIDTH;

function field(p: Vector<2>) {
  // return vec(1 / (p.y) ** 2, p.x)
  // return vec(p.y ** 2, p.x)
  // return vec(1 / p.y ** 2, -1 / p.x ** 2)
  return vec(p.y, -p.x);
  // return vec(Math.E ** p.x, p.y ** 3)
}

function randomPoint() {
  return vec(
    Random.float(CENTER_POINT.x - WIDTH, CENTER_POINT.x + WIDTH),
    Random.float(CENTER_POINT.y - height, CENTER_POINT.y + height)
  );
}

let points: Vector<2>[] = [];
points.push(randomPoint());
let max = 0;
draw(() => {
  c.clear();
  for (let i = 0; i < POINTS - points.length; i++) {
    points.push(randomPoint());
  }

  points = points.filter((p) => {
    const v = field(p);
    p.add(v.norm().scale(SPEED));

    const px = p.clone().add(vec(WIDTH, height)).scale(scale);

    if (!px.within(c.bounds())) {
      return false;
    }

    const mag = p.mag2();
    if (mag > max) max = mag;
    const color = Color.create(Math.round((mag / max) * 255), 100, 10);
    c.circle(px, 2, { fill: color, stroke: color });
    return true;
  });

  load(c);
});
```

3. Press `Ctrl+Shift+Enter`

4. Result

![](./creagen/example.webm)
