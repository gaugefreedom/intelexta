import { useEffect } from 'react';
import { BrowserRouter, Routes, Route, useLocation } from 'react-router-dom';
import Verifier from './components/Verifier';
import PublicReceiptPage from './pages/PublicReceiptPage';

// Component to set document title based on route
function DocumentHead() {
  const location = useLocation();

  useEffect(() => {
    // Set title based on route
    if (location.pathname.startsWith('/r/')) {
      document.title = 'Intelexta Integrity Report - Auditor View';
    } else {
      document.title = 'Intelexta Web Verifier';
    }

    // Set meta description
    const metaDescription = document.querySelector('meta[name="description"]');
    const content = location.pathname.startsWith('/r/')
      ? 'View a verified Intelexta Integrity Report. Auditor view - no login required.'
      : 'Upload CAR archives to verify Intelexta workflow proofs in the browser.';

    if (!metaDescription) {
      const meta = document.createElement('meta');
      meta.name = 'description';
      meta.content = content;
      document.head.appendChild(meta);
    } else {
      metaDescription.setAttribute('content', content);
    }
  }, [location]);

  return null;
}

function App() {
  return (
    <BrowserRouter>
      <DocumentHead />
      <div className="min-h-screen bg-slate-950 text-slate-100">
        <Routes>
          <Route path="/" element={<Verifier />} />
          <Route path="/r/:receiptId" element={<PublicReceiptPage />} />
        </Routes>
      </div>
    </BrowserRouter>
  );
}

export default App;
