import math
import cairosvg

# -----------------------------
# GEOMETRÍA DEL ENGRANAJE
# -----------------------------
def generate_gear_path(cx, cy, r_root, r_tip, teeth=8):
    path_segments = []
    angle_per_tooth = 2 * math.pi / teeth

    for i in range(teeth):
        base_angle = i * angle_per_tooth

        a0 = base_angle
        a1 = base_angle + angle_per_tooth * 0.30
        a2 = base_angle + angle_per_tooth * 0.50
        a3 = base_angle + angle_per_tooth * 0.80

        x0, y0 = cx + r_tip * math.cos(a0), cy + r_tip * math.sin(a0)
        x1, y1 = cx + r_tip * math.cos(a1), cy + r_tip * math.sin(a1)
        x2, y2 = cx + r_root * math.cos(a2), cy + r_root * math.sin(a2)
        x3, y3 = cx + r_root * math.cos(a3), cy + r_root * math.sin(a3)

        if i == 0:
            path_segments.append(f"M {x0:.2f},{y0:.2f}")
        else:
            path_segments.append(f"L {x0:.2f},{y0:.2f}")

        path_segments.append(f"L {x1:.2f},{y1:.2f}")
        path_segments.append(f"L {x2:.2f},{y2:.2f}")
        path_segments.append(f"L {x3:.2f},{y3:.2f}")

    path_segments.append("Z")
    return " ".join(path_segments)


# -----------------------------
# CONFIGURACIÓN
# -----------------------------
cx, cy = 256, 211

pin_path = (
    "M 256,45 C 164.3,45 90,119.3 90,211 "
    "C 90,320 256,495 256,495 "
    "C 256,495 422,320 422,211 "
    "C 422,119.3 347.7,45 256,45 Z"
)

large_hole_path = f"M {cx},{cy-115} A 115,115 0 1,0 {cx},{cy+115} A 115,115 0 1,0 {cx},{cy-115} Z"
inner_hole_path = f"M {cx},{cy-40} A 40,40 0 1,0 {cx},{cy+40} A 40,40 0 1,0 {cx},{cy-40} Z"

gear_path = generate_gear_path(cx, cy, r_root=76, r_tip=100, teeth=8)

combined_d = f"{pin_path} {large_hole_path} {gear_path} {inner_hole_path}"


# -----------------------------
# GENERAR SVG
# -----------------------------
svg_content = f"""<svg xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 512 512">
    <path d="{combined_d}" fill="#D94625" fill-rule="evenodd"/>
</svg>
"""

svg_file = "rustcraper_icon.svg"
with open(svg_file, "w") as f:
    f.write(svg_content)

print(f"SVG generado: {svg_file}")


# -----------------------------
# EXPORTAR PNG MULTI-RESOLUCIÓN
# -----------------------------
sizes = [16, 32, 48, 64, 128, 256, 512, 1024]

for size in sizes:
    png_file = f"icon_{size}.png"

    cairosvg.svg2png(
        url=svg_file,
        write_to=png_file,
        output_width=size,
        output_height=size
    )

    print(f"PNG generado: {png_file}")

print("✔ Exportación completa")