import { createContext, useContext, useState, useCallback, useMemo } from 'react'
import ConfirmModal from '../components/features/AccountManager/ConfirmModal'
import UpdateDialog from '../components/modals/UpdateDialog'

const DialogContext = createContext(null)

/**
 * 全局弹窗 Provider
 */
export function DialogProvider({ children }) {
  const [dialog, setDialog] = useState(null)
  const [resolveRef, setResolveRef] = useState(null)
  // 更新弹窗状态
  const [updateDialog, setUpdateDialog] = useState(null)

  // 显示确认弹窗，返回 Promise<boolean>
  const showConfirm = useCallback((title, message, options = {}) => {
    return new Promise((resolve) => {
      setResolveRef(() => resolve)
      setDialog({
        type: 'confirm',
        title,
        message,
        confirmText: options.confirmText,
        cancelText: options.cancelText,
      })
    })
  }, [])

  // 显示成功弹窗
  const showSuccess = useCallback((title, message, rawData = null) => {
    return new Promise((resolve) => {
      setResolveRef(() => resolve)
      setDialog({
        type: 'success',
        title,
        message,
        rawData,
      })
    })
  }, [])

  // 显示错误弹窗
  const showError = useCallback((title, message) => {
    return new Promise((resolve) => {
      setResolveRef(() => resolve)
      setDialog({
        type: 'error',
        title,
        message,
      })
    })
  }, [])

  // 显示信息弹窗
  const showInfo = useCallback((title, message) => {
    return new Promise((resolve) => {
      setResolveRef(() => resolve)
      setDialog({
        type: 'info',
        title,
        message,
      })
    })
  }, [])

  // 显示更新弹窗
  const showUpdate = useCallback((updateInfo, update) => {
    setUpdateDialog({ updateInfo, update })
  }, [])

  // 关闭更新弹窗
  const closeUpdate = useCallback(() => {
    setUpdateDialog(null)
  }, [])

  const handleConfirm = useCallback(() => {
    if (resolveRef) resolveRef(true)
    setDialog(null)
    setResolveRef(null)
  }, [resolveRef])

  const handleCancel = useCallback(() => {
    if (resolveRef) resolveRef(false)
    setDialog(null)
    setResolveRef(null)
  }, [resolveRef])

  const value = useMemo(() => ({ showConfirm, showSuccess, showError, showInfo, showUpdate, closeUpdate }), [showConfirm, showSuccess, showError, showInfo, showUpdate, closeUpdate])

  return (
    <DialogContext.Provider value={value}>
      {children}
      {dialog && (
        <ConfirmModal
          type={dialog.type}
          title={dialog.title}
          message={dialog.message}
          rawData={dialog.rawData}
          confirmText={dialog.confirmText}
          cancelText={dialog.cancelText}
          onConfirm={handleConfirm}
          onCancel={handleCancel}
        />
      )}
      {updateDialog && (
        <UpdateDialog
          updateInfo={updateDialog.updateInfo}
          update={updateDialog.update}
          onClose={closeUpdate}
        />
      )}
    </DialogContext.Provider>
  )
}

/**
 * 使用全局弹窗 Hook
 */
export function useDialog() {
  const context = useContext(DialogContext)
  if (!context) {
    throw new Error('useDialog must be used within a DialogProvider')
  }
  return context
}
