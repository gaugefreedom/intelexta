import React from 'react';
import { PolicyVersion } from '../lib/api';
import { buttonSecondary, combineButtonStyles } from '../styles/common';

interface PolicyHistoryModalProps {
  versions: PolicyVersion[];
  currentVersion: number;
  onClose: () => void;
}

export default function PolicyHistoryModal({
  versions,
  currentVersion,
  onClose,
}: PolicyHistoryModalProps) {
  return (
    <div
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.7)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
      }}
      onClick={onClose}
    >
      <div
        style={{
          backgroundColor: '#1a1a1a',
          border: '1px solid #333',
          borderRadius: '8px',
          padding: '24px',
          maxWidth: '700px',
          width: '90%',
          maxHeight: '80vh',
          overflow: 'hidden',
          display: 'flex',
          flexDirection: 'column',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
          <h2 style={{ margin: 0, fontSize: '1.2rem' }}>Policy History</h2>
          <button
            onClick={onClose}
            style={{
              background: 'none',
              border: 'none',
              color: '#999',
              fontSize: '1.5rem',
              cursor: 'pointer',
              padding: '0 8px',
            }}
          >
            ×
          </button>
        </div>

        <div style={{ flex: 1, overflowY: 'auto', paddingRight: '8px' }}>
          {versions.length === 0 ? (
            <p style={{ color: '#999', textAlign: 'center', padding: '40px 0' }}>
              No policy versions found
            </p>
          ) : (
            versions.map((version) => {
              const isCurrent = version.version === currentVersion;
              return (
                <div
                  key={version.id}
                  style={{
                    border: isCurrent ? '2px solid #4ade80' : '1px solid #333',
                    borderRadius: '6px',
                    padding: '16px',
                    marginBottom: '12px',
                    backgroundColor: isCurrent ? 'rgba(74, 222, 128, 0.05)' : '#0a0a0a',
                  }}
                >
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                      <span style={{ fontWeight: 'bold', fontSize: '1.1rem' }}>
                        Version {version.version}
                      </span>
                      {isCurrent && (
                        <span
                          style={{
                            backgroundColor: '#4ade80',
                            color: '#000',
                            padding: '2px 8px',
                            borderRadius: '4px',
                            fontSize: '0.75rem',
                            fontWeight: 'bold',
                          }}
                        >
                          CURRENT
                        </span>
                      )}
                    </div>
                    <span style={{ color: '#999', fontSize: '0.85rem' }}>
                      {new Date(version.createdAt).toLocaleString()}
                    </span>
                  </div>

                  {version.createdBy && (
                    <div style={{ fontSize: '0.85rem', color: '#999', marginBottom: '8px' }}>
                      by {version.createdBy}
                    </div>
                  )}

                  {version.changeNotes && (
                    <div
                      style={{
                        fontSize: '0.9rem',
                        color: '#ccc',
                        marginBottom: '12px',
                        fontStyle: 'italic',
                        padding: '8px',
                        backgroundColor: 'rgba(255, 255, 255, 0.05)',
                        borderRadius: '4px',
                      }}
                    >
                      "{version.changeNotes}"
                    </div>
                  )}

                  <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px', fontSize: '0.9rem' }}>
                    <div>
                      <span style={{ color: '#999' }}>Token Budget:</span>{' '}
                      <span style={{ color: '#fff' }}>{version.policy.budgetTokens.toLocaleString()}</span>
                    </div>
                    <div>
                      <span style={{ color: '#999' }}>USD Budget:</span>{' '}
                      <span style={{ color: '#fff' }}>${version.policy.budgetUsd.toFixed(2)}</span>
                    </div>
                    <div>
                      <span style={{ color: '#999' }}>Nature Cost:</span>{' '}
                      <span style={{ color: '#fff' }}>{version.policy.budgetNatureCost.toFixed(1)}</span>
                    </div>
                    <div>
                      <span style={{ color: '#999' }}>Network:</span>{' '}
                      <span style={{ color: version.policy.allowNetwork ? '#4ade80' : '#f87171' }}>
                        {version.policy.allowNetwork ? 'Allowed' : 'Denied'}
                      </span>
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>

        <div style={{ marginTop: '20px', display: 'flex', justifyContent: 'flex-end' }}>
          <button type="button" onClick={onClose} style={combineButtonStyles(buttonSecondary)}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
