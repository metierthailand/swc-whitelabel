import React from "react";

const _Heading: React.FC = ({ children }) => <h1>{children}</h1>;

// whitelabel key=Heading
export const Heading = _Heading;

const _Layout: React.FC = ({ children }) => (
  <div className="bg-red-500">{children}</div>
);

// whitelabel
export const Layout = _Layout;

const _Footer: React.FC = () => <p>Contact Us!</p>;

// whitelabel
export const Footer = _Footer;

const _Button: React.FC = ({ children }) => <button>{children}</button>;

// whitelabel
export const Button = _Button;

const App = () => (
  <Layout>
    <Heading>
      Hello, World! <Button>Click me!</Button>
    </Heading>

    <Footer />
  </Layout>
);
