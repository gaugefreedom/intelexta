import { useEffect } from 'react';
import Verifier from './components/Verifier';

function App() {
  useEffect(() => {
    document.title = 'IntelexTA Web Verifier';
    const metaDescription = document.querySelector('meta[name="description"]');
    if (!metaDescription) {
      const meta = document.createElement('meta');
      meta.name = 'description';
      meta.content = 'Upload CAR archives to verify IntelexTA workflow proofs in the browser.';
      document.head.appendChild(meta);
    } else {
      metaDescription.setAttribute(
        'content',
        'Upload CAR archives to verify IntelexTA workflow proofs in the browser.'
      );
    }
  }, []);

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100">
      <Verifier />
    </div>
  );
}

export default App;
