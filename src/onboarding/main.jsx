import "../diagnostics-bootstrap.js";
import {reactErrorHandlers} from "../diagnostics.js";
import React from 'react';
import ReactDOM from 'react-dom/client';
import OnboardingGuide from './OnboardingGuide.jsx';
import './onboarding.css';

ReactDOM.createRoot(document.getElementById('root'), reactErrorHandlers).render(
  <React.StrictMode>
    <OnboardingGuide />
  </React.StrictMode>,
);
