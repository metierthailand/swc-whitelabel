import {
  ecommerce_product_bannerImage,
  ecommerce_product_intro,
} from "./_components/products-banner-section";

const Page = () => (
  <div
    style={{
      backgroundImage: ecommerce_product_bannerImage,
    }}
  >
    <p>{ecommerce_product_intro}</p>
  </div>
);
