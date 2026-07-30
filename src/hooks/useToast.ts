import { useState, useCallback, useRef } from 'react';

export interface ToastMessage {
  id: string;
  type: 'success' | 'error' | 'warning' | 'info';
  message: string;
  duration?: number;
}

export function useToast() {
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  // 关键修复（v1.0.36）：用自增计数器替代 Date.now()，避免同毫秒内多个 Toast id 撞车
  const idCounter = useRef(0);

  const addToast = useCallback((
    type: ToastMessage['type'],
    message: string,
    duration?: number
  ) => {
    idCounter.current += 1;
    const id = `toast-${idCounter.current}`;
    setToasts((prev) => [...prev, { id, type, message, duration }]);
  }, []);

  const removeToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  return { toasts, addToast, removeToast };
}
