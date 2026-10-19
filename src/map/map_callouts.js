// Labels on the map where each side of the scale gathers: a pill outside
// the circle, toward the nearer edge, with a line to it.
function drawCallouts(list) {
  const drawn = [];
  list.forEach((callout) => {
    const n = nodes[callout.node];
    const right = n.x >= SIZE / 2;
    const kind = callout.heat < 0 ? "risk" : "good";
    svg.querySelector(`circle[data-node="${callout.node}"]`).classList.add("marked", kind);
    const group = svgEl("g", { class: "callout " + kind });
    const text = svgEl("text", { "font-size": 15 });
    text.textContent = callout.label;
    group.append(svgEl("line", {}), svgEl("rect", { rx: 14 }), text);
    svg.append(group);
    const width = text.getComputedTextLength() + 34;
    const edgeX = n.x + (right ? n.r * 0.7 : -n.r * 0.7);
    const edgeY = n.y - n.r * 0.7;
    const x = right ? Math.min(edgeX + 40, SIZE - width - 4) : Math.max(edgeX - 40 - width, 4);
    const y = Math.max(edgeY - 40, 18);
    group.querySelector("line").setAttribute("x1", edgeX);
    group.querySelector("line").setAttribute("y1", edgeY);
    group.querySelector("line").setAttribute("x2", right ? x : x + width);
    group.querySelector("line").setAttribute("y2", y);
    const rect = group.querySelector("rect");
    [["x", x], ["y", y - 14], ["width", width], ["height", 28]].forEach(([k, v]) => rect.setAttribute(k, v));
    text.setAttribute("x", x + width / 2);
    text.setAttribute("y", y + 5);
    drawn.push(group);
  });
  return drawn;
}

