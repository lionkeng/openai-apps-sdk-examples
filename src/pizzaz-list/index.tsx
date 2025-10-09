import { createRoot } from "react-dom/client";
import PizzazListApp from "./PizzazListApp";

export { PizzazListApp } from "./PizzazListApp";
export default PizzazListApp;

if (typeof document !== "undefined") {
  const container = document.getElementById("pizzaz-list-root");
  if (container) {
    createRoot(container).render(<PizzazListApp />);
  } else if (import.meta.env?.PROD) {
    console.error(
      "[pizzaz-list] Failed to mount widget: no element with id 'pizzaz-list-root' found."
    );
  }
}
