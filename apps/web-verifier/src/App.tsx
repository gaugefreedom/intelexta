import { useEffect } from 'react';
import { BrowserRouter, Routes, Route, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import Verifier from './components/Verifier';
import PublicReceiptPage from './pages/PublicReceiptPage';

function DocumentHead() {
  const location = useLocation();
  const { t } = useTranslation();

  useEffect(() => {
    const isAuditor = location.pathname.startsWith('/r/');
    document.title = isAuditor ? t('page_title_auditor') : t('page_title_verifier');

    const metaDescription = document.querySelector('meta[name="description"]');
    const content = isAuditor ? t('page_meta_auditor') : t('page_meta_verifier');

    if (!metaDescription) {
      const meta = document.createElement('meta');
      meta.name = 'description';
      meta.content = content;
      document.head.appendChild(meta);
    } else {
      metaDescription.setAttribute('content', content);
    }
  }, [location, t]);

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
