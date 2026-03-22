import { Heading } from "./_components/heading";

/**
 * The most formal `whitelabel` marker
 */
// whitelabel: for=variant1, key=BG_COLOR
export const variant1_bgClassname: string = "bg-red-100";

/**
 * If `for` is omitted, it will default to your `default_target` config
 */
// whitelabel
export const BG_COLOR: string = "bg-red-200";

/**
 * The most natural `whitelabel` marker
 * `as` works the same way as `key`, excepts you can omits `=`.
 */
// whitelabel for 'variant2' as 'BG_COLOR'
export const variant2_bgClassname: string = "bg-red-300";

/**
 * Or
 * (Not recommended, hard to read)
 */
// whitelabel for variant3 as BG_COLOR
export const variant3_bgClassname: string = "bg-red-400";

/**
 * Or
 */
// whitelabel for 'variant4' as='BG_COLOR'
export const variant4_bgClassname: string = "bg-red-400";

const Homepage = () => (
  <div className={`h-full w-full ${BG_COLOR}`}>
    <Heading />
  </div>
);

export default Homepage;
