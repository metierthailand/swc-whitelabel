import React from "react";

// whitelabel for=mar, key=Heading
export const MarHeading: React.FC = ({ children }) => <h2>{children}</h2>;

// whitelabel for=mar
export const Layout: React.FC = ({ children }) => (
  <div className="bg-green-500">{children}</div>
);

// whitelabel for=mar
export const Footer: React.FC = () => <p>Contact Them!</p>;

// whitelabel for=mar
export const Button: React.FC = ({ children }) => (
  <button className="text-lg">{children}</button>
);
