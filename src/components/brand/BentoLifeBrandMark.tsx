import bentoLifeBrandIcon from "../../../assets/v3/bentolife-light-v3.png";

export function BentoLifeBrandMark() {
  return (
    <div
      aria-label="BentoLife"
      className="flex size-10 shrink-0 items-center justify-center overflow-hidden rounded-md bg-primary text-primary-foreground"
      role="img"
    >
      <img
        alt=""
        aria-hidden="true"
        className="size-8 object-contain"
        data-brand-icon="bentolife"
        src={bentoLifeBrandIcon}
      />
    </div>
  );
}
