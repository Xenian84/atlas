interface Props {
  width?: string | number;
  height?: string | number;
  style?: React.CSSProperties;
  className?: string;
}

export default function Skeleton({ width = '100%', height = 16, style, className }: Props) {
  return (
    <span
      className={`skeleton ${className ?? ''}`}
      style={{ display: 'block', width, height, ...style }}
    />
  );
}
