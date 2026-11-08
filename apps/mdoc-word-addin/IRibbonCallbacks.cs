using System.Runtime.InteropServices;
using Microsoft.Office.Core;

namespace MdocWordAddIn
{
    /// <summary>
    /// Dispatch-Schnittstelle für Ribbon-Callbacks (onAction-Namen).
    /// Muss die Default-COM-Schnittstelle sein, damit Word die Methoden findet.
    /// </summary>
    [ComVisible(true)]
    [Guid("8F3C2E91-4B6A-4D11-9C2E-1A7B6D5E4F30")]
    [InterfaceType(ComInterfaceType.InterfaceIsIDispatch)]
    public interface IRibbonCallbacks
    {
        void OnRibbonLoad(IRibbonUI ribbonUi);

        void OnMapStyles(IRibbonControl control);

        void OnOpenEditor(IRibbonControl control);

        void OnInsertMarkdown(IRibbonControl control);

        void OnCopyMarkdown(IRibbonControl control);
    }
}
