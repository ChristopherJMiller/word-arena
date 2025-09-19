import React from "react";

interface SessionConflictModalProps {
  isOpen: boolean;
  onForceLogin: () => void;
  onCancel: () => void;
  message?: string;
}

export const SessionConflictModal: React.FC<SessionConflictModalProps> = ({
  isOpen,
  onForceLogin,
  onCancel,
  message = "You already have an active session in another browser.",
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-gray-800 rounded-lg p-6 max-w-md w-full mx-4">
        <h2 className="text-xl font-bold text-white mb-4">Active Session Detected</h2>
        
        <p className="text-gray-300 mb-6">
          {message}
        </p>
        
        <p className="text-gray-400 mb-6 text-sm">
          You can either disconnect the other session and continue here, or close this window
          and return to your existing session.
        </p>

        <div className="flex gap-3 justify-end">
          <button
            onClick={onCancel}
            className="px-4 py-2 bg-gray-700 text-white rounded hover:bg-gray-600 transition-colors"
          >
            Close Window
          </button>
          <button
            onClick={onForceLogin}
            className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
          >
            Disconnect Other Session
          </button>
        </div>
      </div>
    </div>
  );
};