// Patterns that tell the sides of the scale apart without colour, for print
// and colour blindness: lines over the files at risk, dots over the good
// ones, shown when the reader asks for them.

// The side of the scale a heat stands on, none when it is grey.
function side(heat) {
  if (heat === undefined || Math.abs(heat) < 0.05) return null;
  return heat < 0 ? "risk" : "good";
}

// The two patterns, in the defs of the map.
function drawPatterns() {
  const defs = svgEl("defs", {});
  const marks = {
    risk: svgEl("path", { d: "M-2 2L2 -2M0 8L8 0M6 10L10 6" }),
    good: svgEl("circle", { cx: 4, cy: 4, r: 1.3 }),
  };
  const patterns = Object.entries(marks).map(([kind, mark]) => {
    const pattern = svgEl("pattern", { id: "hatch-" + kind, patternUnits: "userSpaceOnUse", viewBox: "0 0 8 8" });
    pattern.append(mark);
    defs.append(pattern);
    return pattern;
  });
  svg.append(defs);
  document.getElementById("patterns").addEventListener("change", (event) => {
    document.body.classList.toggle("patterns", event.target.checked);
  });
  return patterns;
}

// A file's pattern, over its circle.
function hatch(node) {
  const kind = side(node.heat);
  if (!kind) return;
  svg.append(svgEl("circle", { cx: node.x, cy: node.y, r: node.r, class: "hatch", fill: `url(#hatch-${kind})` }));
}

// The same size on screen at any zoom.
function scalePatterns(patterns, k) {
  patterns.forEach((p) => ["width", "height"].forEach((a) => p.setAttribute(a, 8 / k)));
}
