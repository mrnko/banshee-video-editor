import type { SpendPoint } from "../lib/types";

export function SpendChart({ points }: { points: SpendPoint[] }) {
  const width = 760, height = 220, padX = 18, padY = 22;
  const max = Math.max(...points.map(p => p.value), 0.01);
  const coords = points.map((point, index) => ({
    ...point,
    x: padX + index * ((width - padX * 2) / Math.max(1, points.length - 1)),
    y: height - padY - (point.value / max) * (height - padY * 2)
  }));
  const line = coords.map((p, i) => `${i ? "L" : "M"}${p.x},${p.y}`).join(" ");
  const area = `${line} L${coords.at(-1)?.x ?? 0},${height - padY} L${coords[0]?.x ?? 0},${height - padY} Z`;
  return <div className="chart-wrap">
    <svg viewBox={`0 0 ${width} ${height}`} role="img" aria-label="Графік витрат за сім днів">
      <defs><linearGradient id="chart-area" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stopColor="#9a63ff" stopOpacity=".34"/><stop offset="1" stopColor="#9a63ff" stopOpacity="0"/></linearGradient></defs>
      {[.25,.5,.75,1].map(v => <line key={v} x1={padX} x2={width-padX} y1={height-padY-v*(height-padY*2)} y2={height-padY-v*(height-padY*2)} stroke="#ffffff0b" />)}
      <path d={area} fill="url(#chart-area)"/><path d={line} fill="none" stroke="#a97bff" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"/>
      {coords.map(p => <g key={p.label}><circle cx={p.x} cy={p.y} r="4" fill="#d7c0ff" stroke="#6d35d3" strokeWidth="3"/><text x={p.x} y={height-2} textAnchor="middle" className="chart-label">{p.label}</text></g>)}
    </svg>
  </div>;
}

