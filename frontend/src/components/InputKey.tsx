import * as React from "react";
import { cn } from "@/lib/utils";
import { Eye, EyeClosed } from "lucide-react";

type InputProps = React.PropsWithChildren<React.ComponentProps<"input">> & {
  icon?: React.ReactNode;
};

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ children, icon, className, type, ...props }, ref) => {
    const classes = cn(
      "flex h-10 w-full gap-x-2 rounded-md border border-input bg-background px-3 py-2 text-base has-[:focus-visible]:outline-none has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring has-[:focus-visible]:ring-offset-2 has-[:disabled]:cursor-not-allowed has-[:disabled]:opacity-50 md:text-sm",
      className
    );
    return (
      <div className={classes}>
        {icon}
        <input
          type={type}
          className="flex-1 h-full bg-background outline-none file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground disabled:cursor-not-allowed"
          ref={ref}
          {...props}
        />
        {children}
      </div>
    );
  }
);
Input.displayName = "Input";

const InputKey = React.forwardRef<
  HTMLInputElement,
  Omit<React.ComponentProps<"input">, "type">
>((props, ref) => {
  const [showPassword, setShowPassword] = React.useState(false);
  const togglePasswordVisibility = () => setShowPassword(!showPassword);
  return (
    <Input ref={ref} {...props} type={showPassword ? "text" : "password"}>
      <div className="flex items-center gap-x-1 self-center">
        {showPassword ? (
          <EyeClosed
            className="cursor-pointer"
            onClick={togglePasswordVisibility}
            size={20}
          />
        ) : (
          <Eye
            className="cursor-pointer"
            onClick={togglePasswordVisibility}
            size={20}
          />
        )}
      </div>
    </Input>
  );
});
InputKey.displayName = "InputPassword";

export { Input, InputKey };
