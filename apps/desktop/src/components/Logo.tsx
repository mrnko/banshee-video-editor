import mark from "../assets/banshee-mark.svg";

export function Logo({ compact = false }: { compact?: boolean }) {
  return <div className="logo"><img src={mark} alt="Banshee" /><div>{!compact && <><strong>Banshee</strong><span>Video Editor</span></>}</div></div>;
}

