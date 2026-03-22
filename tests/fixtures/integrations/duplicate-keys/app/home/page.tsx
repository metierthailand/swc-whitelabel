import { FC } from "react";

/**
 * Duplicated with `./_components/heading.tsx`
 */
// whitelabel for def
export const Heading: FC = () => <h1>Heading</h1>;

const BG_COLOR: string = "bg-red-200";

const Homepage = () => (
  <div className={`h-full w-full ${BG_COLOR}`}>
    <Heading />
  </div>
);

export default Homepage;
