import { Heading } from "./_components/heading";

// whitelabel: key=BG_COLOR
export const bgClassname: string = "bg-red-500";

// whitelabel: for=variant1, key=BG_COLOR
export const variant1_bgClassname: string = "bg-green-500";

const Homepage = () => (
  <div className={`h-full w-full ${bgClassname}`}>
    <Heading />
  </div>
);

export default Homepage;
