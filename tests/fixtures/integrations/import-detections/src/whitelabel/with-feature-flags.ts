import whitelabel, { type WhitelabelConfig } from ".";

export const withFeatureFlags = <TArgs, TReturnType>(
  Fn: (args: TArgs) => TReturnType,
  pred: (config: WhitelabelConfig) => boolean,
) => {
  return function WithFeatureFlags(props: TArgs): TReturnType | null {
    return pred(whitelabel) ? Fn(props) : null;
  };
};
