import "./App.css";
import { TowerView } from "./TowerView";
import { WelcomeModal } from "./WelcomeModal";
import { utmSpec } from "./parseSpec";

function App() {
  return (
    <>
      <WelcomeModal />
      <TowerView stateDescriptions={utmSpec.stateDescriptions} />
    </>
  );
}

export default App;
