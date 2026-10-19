// The dependency matrix: rows and columns are the directories and files
// shown, a cell counts the files of its column that use files of its row.
const data = JSON.parse(document.getElementById("data").textContent);
const nodes = data.nodes;
const expanded = new Set(data.expanded);
const table = document.getElementById("matrix");
const details = document.getElementById("details");
const hint = details.innerHTML;
let selected = null;
// The rows and cells shown, for the details of a cell.
let shown = { rows: [], cells: new Map() };

const isDir = (id) => nodes[id].file === null;

function filesUnder(id) {
  const found = [];
  const pending = [id];
  while (pending.length) {
    const n = nodes[pending.pop()];
    if (n.file !== null) found.push(n.file);
    pending.push(...n.children);
  }
  return found;
}

// The lines shown: expanded directories head their children.
function lines() {
  const out = [];
  const walk = (id, level) => {
    const group = isDir(id) && expanded.has(id);
    out.push({ node: id, level, group });
    if (group) nodes[id].children.forEach((c) => walk(c, level + 1));
  };
  nodes[0].children.forEach((c) => walk(c, 0));
  return out;
}

// The cells of the rows, by "row,column": the row is used, the column uses it.
function cellsOf(rows) {
  const rowOf = [];
  rows.forEach((line, r) => filesUnder(line.node).forEach((f) => (rowOf[f] = r)));
  const cells = new Map();
  data.edges.forEach(([from, to, strong, cyclic], i) => {
    const row = rowOf[to];
    const col = rowOf[from];
    if (row === col) return;
    const key = row + "," + col;
    const cell = cells.get(key) || { count: 0, strong: false, cycle: false, edges: [] };
    cell.count += 1;
    cell.strong ||= strong;
    cell.cycle ||= cyclic;
    cell.edges.push(i);
    cells.set(key, cell);
  });
  return cells;
}

function mark(cell, above) {
  if (cell.cycle) return "cycle";
  if (!cell.strong) return "weak";
  return above ? "layering" : "plain";
}

const WHY = {
  cycle: "in a cycle",
  layering: "against the layering",
  weak: "type-only or deferred",
  plain: "below the diagonal",
};

function el(tag, attrs, text) {
  const e = document.createElement(tag);
  Object.entries(attrs || {}).forEach(([k, v]) => e.setAttribute(k, v));
  if (text !== undefined) e.textContent = text;
  return e;
}

function label(line, number) {
  const n = nodes[line.node];
  const th = el("th", { class: "label", title: n.path + (isDir(line.node) ? "/" : "") });
  th.style.paddingLeft = 6 + line.level * 14 + "px";
  th.append(el("span", { class: "num" }, number === null ? "" : String(number)));
  const toggle = isDir(line.node) ? (line.group ? "▾" : "▸") : "";
  th.append(el("span", { class: "toggle", "data-toggle": line.node }, toggle));
  th.append(document.createTextNode(n.name));
  return th;
}

function shade(count, max) {
  const share = max > 1 ? Math.log(count) / Math.log(max) : 1;
  return `color-mix(in srgb, var(--accent) ${Math.round(12 + share * 48)}%, transparent)`;
}

function cellTd(cell, r, c) {
  if (r === c) return el("td", { class: "diagonal" });
  if (!cell) return el("td");
  const kind = mark(cell, r < c);
  const td = el("td", { class: "cell " + kind, "data-cell": r + "," + c }, String(cell.count));
  return td;
}

function render() {
  const all = lines();
  const rows = all.filter((l) => !l.group);
  const cells = cellsOf(rows);
  const max = Math.max(1, ...[...cells.values()].map((c) => c.count));
  table.replaceChildren();
  const head = el("tr");
  head.append(el("th", { class: "label" }));
  rows.forEach((_, c) => head.append(el("th", { class: "col", "data-col": c }, String(c + 1))));
  table.appendChild(el("thead")).append(head);
  const body = table.appendChild(el("tbody"));
  let r = 0;
  all.forEach((line) => {
    const tr = body.appendChild(el("tr", { class: line.group ? "group" : "row" }));
    if (line.group) {
      tr.append(label(line, null));
      tr.append(el("td", { colspan: rows.length }));
      return;
    }
    tr.dataset.row = r;
    tr.dataset.node = line.node;
    tr.append(label(line, r + 1));
    rows.forEach((_, c) => {
      const cell = cells.get(r + "," + c);
      const td = tr.appendChild(cellTd(cell, r, c));
      if (cell && td.classList.contains("plain")) td.style.background = shade(cell.count, max);
    });
    r += 1;
  });
  summarize(rows, cells);
  highlight(rows, cells);
  shown = { rows, cells };
}

function summarize(rows, cells) {
  const above = [...cells.keys()].filter((k) => {
    const [r, c] = k.split(",").map(Number);
    return r < c;
  }).length;
  const cycles = data.edges.filter((e) => e[3]).length;
  document.getElementById("summary").textContent =
    `${data.files.length} files, ${data.edges.length} uses, ${cycles} in cycles; ` +
    `${rows.length} rows shown, ${above} cells above the diagonal`;
}

// What the selected row uses (its column) and what uses it (its row).
function highlight(rows, cells) {
  const r = rows.findIndex((l) => l.node === selected);
  if (r < 0) return;
  const tr = table.querySelector(`tr[data-row="${r}"]`);
  tr.classList.add("selected");
  table.querySelector(`th[data-col="${r}"]`).classList.add("selected");
  cells.forEach((_, key) => {
    const [row, col] = key.split(",").map(Number);
    const td = table.querySelector(`td[data-cell="${key}"]`);
    if (col === r) {
      td.classList.add("uses");
      table.querySelector(`tr[data-row="${row}"] th.label`).classList.add("uses");
    }
    if (row === r) {
      td.classList.add("used");
      table.querySelector(`tr[data-row="${col}"] th.label`).classList.add("used");
    }
  });
}

function showCell(key) {
  const [r, c] = key.split(",").map(Number);
  const cell = shown.cells.get(key);
  const name = (i) => nodes[shown.rows[i].node].path + (isDir(shown.rows[i].node) ? "/" : "");
  details.replaceChildren(el("h2", {}, `${name(c)} → ${name(r)}`));
  details.append(el("p", { class: "hint" }, `${cell.count} ${cell.count === 1 ? "use" : "uses"}, ${WHY[mark(cell, r < c)]}`));
  const list = details.appendChild(el("ul"));
  cell.edges.forEach((i) => {
    const [from, to, strong, cyclic] = data.edges[i];
    const note = cyclic ? " (cycle)" : strong ? "" : " (type-only or deferred)";
    list.append(el("li", {}, `${data.files[from]} → ${data.files[to]}${note}`));
  });
  table.querySelectorAll("td.picked").forEach((td) => td.classList.remove("picked"));
  table.querySelector(`td[data-cell="${key}"]`).classList.add("picked");
}

table.addEventListener("click", (event) => {
  const toggle = event.target.closest("[data-toggle]");
  const td = event.target.closest("td[data-cell]");
  const tr = event.target.closest("tr[data-node]");
  if (toggle && toggle.textContent) {
    const id = Number(toggle.dataset.toggle);
    expanded.has(id) ? expanded.delete(id) : expanded.add(id);
    reshape();
  } else if (td) {
    showCell(td.dataset.cell);
  } else if (tr && event.target.closest("th.label")) {
    const id = Number(tr.dataset.node);
    selected = selected === id ? null : id;
    render();
  }
});

// Rows change: the cell shown in the details may no longer exist.
function reshape() {
  details.innerHTML = hint;
  render();
}

document.getElementById("expand").addEventListener("click", () => {
  nodes.forEach((n, id) => isDir(id) && n.children.length && expanded.add(id));
  reshape();
});
document.getElementById("collapse").addEventListener("click", () => {
  expanded.clear();
  reshape();
});

render();
