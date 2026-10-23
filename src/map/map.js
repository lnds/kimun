// The code map: directories as circles holding their files, zoomed by
// moving the view box of the SVG.
const data = JSON.parse(document.getElementById("data").textContent);
const nodes = data.nodes;
const svg = document.getElementById("map");
const tip = document.getElementById("tip");
const NS = "http://www.w3.org/2000/svg";
const SIZE = 1000;
let focus = 0;
let view = [0, 0, SIZE];

function fill(node) {
  if (node.heat === undefined) return "var(--nodata)";
  const share = Math.round(35 + Math.abs(node.heat) * 65);
  if (Math.abs(node.heat) < 0.05) return "var(--neutral)";
  const colour = node.heat < 0 ? "var(--bad)" : "var(--good)";
  return `color-mix(in srgb, ${colour} ${share}%, var(--neutral))`;
}

// The side of the scale a heat stands on, none when it is grey.
function side(heat) {
  if (heat === undefined || Math.abs(heat) < 0.05) return null;
  return heat < 0 ? "risk" : "good";
}

function svgEl(tag, attrs) {
  const e = document.createElementNS(NS, tag);
  Object.entries(attrs).forEach(([k, v]) => e.setAttribute(k, v));
  return e;
}

// Patterns that tell the sides apart without colour: lines for risk, dots
// for good, drawn over the files when asked for.
const defs = svgEl("defs", {});
const hatches = [
  ["risk", svgEl("path", { d: "M-2 2L2 -2M0 8L8 0M6 10L10 6" })],
  ["good", svgEl("circle", { cx: 4, cy: 4, r: 1.3 })],
].map(([kind, mark]) => {
  const pattern = svgEl("pattern", { id: "hatch-" + kind, patternUnits: "userSpaceOnUse", viewBox: "0 0 8 8" });
  pattern.append(mark);
  defs.append(pattern);
  return pattern;
});
svg.append(defs);

const labels = [];
nodes.forEach((node, i) => {
  const circle = svgEl("circle", { cx: node.x, cy: node.y, r: node.r, class: node.dir ? "dir" : "file" });
  if (!node.dir) circle.style.fill = fill(node);
  circle.dataset.node = i;
  svg.append(circle);
  const kind = node.dir ? null : side(node.heat);
  if (kind) svg.append(svgEl("circle", { cx: node.x, cy: node.y, r: node.r, class: "hatch", fill: `url(#hatch-${kind})` }));
});
nodes.forEach((node, i) => {
  if (!node.dir || i === 0) return;
  const text = svgEl("text", { x: node.x });
  text.textContent = node.name;
  labels.push({ node, text });
  svg.append(text);
});

// Whether a node lies under the directory in focus.
function inside(node) {
  for (let n = node; n.parent !== null; n = nodes[n.parent]) {
    if (n.parent === focus) return true;
  }
  return focus === 0;
}

// Directory names large enough at the present zoom, near the top of their circle.
function relabel() {
  const k = SIZE / view[2];
  hatches.forEach((p) => ["width", "height"].forEach((a) => p.setAttribute(a, 8 / k)));
  labels.forEach(({ node, text }) => {
    const near = node.depth - nodes[focus].depth <= 2;
    const shown = node.r * k > 34 && near && inside(node);
    text.style.display = shown ? "" : "none";
    const size = Math.min(16, Math.max(10, node.r * k * 0.18)) / k;
    text.setAttribute("font-size", size);
    text.setAttribute("y", node.y - node.r + size * 1.6);
  });
  const crumb = nodes[focus];
  document.getElementById("crumb").textContent = focus === 0 ? "" : crumb.path + "/";
  document.getElementById("up").disabled = focus === 0;
  callouts.forEach((c) => (c.style.display = focus === 0 ? "" : "none"));
}

function zoomTo(i) {
  focus = i;
  const n = nodes[i];
  const target = i === 0 ? [0, 0, SIZE] : [n.x - n.r, n.y - n.r, n.r * 2];
  const start = view.slice();
  const began = performance.now();
  const step = (now) => {
    const t = Math.min(1, (now - began) / 450);
    const ease = t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
    view = start.map((v, j) => v + (target[j] - v) * ease);
    svg.setAttribute("viewBox", `${view[0]} ${view[1]} ${view[2]} ${view[2]}`);
    if (t < 1) requestAnimationFrame(step);
    else relabel();
  };
  requestAnimationFrame(step);
}

// The directory to zoom into: the one clicked, or the one holding the file clicked.
function zoomable(i) {
  const n = nodes[i];
  return n.dir ? i : n.parent;
}

svg.addEventListener("click", (event) => {
  const target = event.target.closest("[data-node]");
  if (!target) return zoomTo(nodes[focus].parent ?? 0);
  const i = zoomable(Number(target.dataset.node));
  zoomTo(i === focus ? nodes[focus].parent ?? 0 : i);
});
document.getElementById("up").addEventListener("click", () => zoomTo(nodes[focus].parent ?? 0));
document.getElementById("patterns").addEventListener("change", (event) => {
  document.body.classList.toggle("patterns", event.target.checked);
});

svg.addEventListener("mousemove", (event) => {
  const target = event.target.closest("[data-node]");
  if (!target) return (tip.hidden = true);
  const n = nodes[Number(target.dataset.node)];
  tip.replaceChildren();
  const name = document.createElement("b");
  name.textContent = n.path + (n.dir ? "/" : "");
  const lines = document.createElement("span");
  lines.textContent = `${n.lines.toLocaleString()} lines of code`;
  tip.append(name, lines);
  if (n.label) {
    const value = document.createElement("span");
    value.textContent = n.label;
    tip.append(value);
  }
  tip.hidden = false;
  tip.style.left = Math.min(event.clientX + 14, window.innerWidth - 370) + "px";
  tip.style.top = event.clientY + 14 + "px";
});
svg.addEventListener("mouseleave", () => (tip.hidden = true));

const legend = document.getElementById("legend");
legend.append(document.createTextNode(data.metric + ":"));
data.legend.forEach((key) => {
  const swatch = document.createElement("i");
  swatch.className = ["swatch", side(key.heat)].filter(Boolean).join(" ");
  swatch.style.backgroundColor = fill({ heat: key.heat });
  legend.append(swatch, document.createTextNode(key.label));
});
// The share of the lines on each side of the scale.
document.getElementById("summary").textContent = data.summary
  .map((s) => `${Math.round(s.share * 100)}% ${s.label}`)
  .join(" · ");

const callouts = drawCallouts(data.callouts);

const none = document.createElement("i");
none.className = "swatch";
none.style.backgroundColor = "var(--nodata)";
legend.append(none, document.createTextNode("no value"));
relabel();
