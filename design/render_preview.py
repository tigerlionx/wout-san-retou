# Rasterise the PDF pages to design/preview/pN.png (white background) to look at them.
import sys, os
import pypdfium2 as pdfium
here = os.path.dirname(os.path.abspath(__file__))
pdf = pdfium.PdfDocument(os.path.join(here, 'wout-san-retou-building.pdf'))
os.makedirs(os.path.join(here, 'preview'), exist_ok=True)
width = int(sys.argv[1]) if len(sys.argv) > 1 else 2400
for i in range(len(pdf)):
    w, h = pdf[i].get_size()
    pdf[i].render(scale=width / max(w, h), fill_color=(255, 255, 255, 255)).to_pil().save(os.path.join(here, 'preview', f'p{i+1}.png'))
print(len(pdf), 'pages')
