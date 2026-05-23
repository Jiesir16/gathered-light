import { AdminApp } from "./components/AdminApp";
import { PublicGallery } from "./components/PublicGallery";

export function App() {
  const path = window.location.pathname;
  if (path.startsWith("/admin")) {
    return <AdminApp />;
  }
  return <PublicGallery />;
}
