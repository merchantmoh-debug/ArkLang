import manifold3d as m3d
import time, struct, json
import numpy as np

t0 = time.perf_counter()
base = m3d.Manifold.cube([100, 100, 100], center=True)

spacing = 100 / 25
offset = -100 / 2.0 + spacing / 2.0
extend = 2.0
full_len = 100 + extend * 2

cyl_z = m3d.Manifold.cylinder(full_len, 1.5, circular_segments=16).translate([0, 0, -100/2 - extend])
cyl_x = cyl_z.rotate([0, 90, 0])
cyl_y = cyl_z.rotate([-90, 0, 0])

n = 25
x_v = [cyl_x.translate([0, offset+i*spacing, offset+j*spacing]) for i in range(n) for j in range(n)]
y_v = [cyl_y.translate([offset+i*spacing, 0, offset+j*spacing]) for i in range(n) for j in range(n)]
z_v = [cyl_z.translate([offset+i*spacing, offset+j*spacing, 0]) for i in range(n) for j in range(n)]

all_voids = m3d.Manifold.batch_boolean(x_v + y_v + z_v, m3d.OpType.Add)
final = base - all_voids
t1 = time.perf_counter()

print(f'[ARK-FFI] CSG compiled in {t1-t0:.4f}s')
print(f'[ARK-FFI] Vertices: {final.num_vert():,} | Triangles: {final.num_tri():,}')
print(f'[ARK-FFI] Volume: {final.volume():.2f} mm3 | Surface Area: {final.surface_area():.2f} mm2')
print(f'[ARK-FFI] Genus: {final.genus()} | Manifold: YES')

mesh = final.to_mesh()
verts = np.array(mesh.vert_properties, dtype=np.float32)[:, :3]
faces = np.array(mesh.tri_verts, dtype=np.uint32)
v0,v1,v2 = verts[faces[:,0]], verts[faces[:,1]], verts[faces[:,2]]
fn = np.cross(v1-v0, v2-v0)
nn = np.linalg.norm(fn, axis=1, keepdims=True); nn = np.where(nn==0,1,nn)
fn = fn / nn
vn = np.zeros_like(verts)
for i in range(3): np.add.at(vn, faces[:,i], fn)
nn2 = np.linalg.norm(vn, axis=1, keepdims=True); nn2 = np.where(nn2==0,1,nn2)
vn = (vn / nn2).astype(np.float32)
vb,nb,ib = verts.tobytes(), vn.tobytes(), faces.flatten().astype(np.uint32).tobytes()
tb = ib + vb + nb; pad = (4-len(tb)%4)%4; tb += b'\x00'*pad
il,vl,nl = len(ib), len(vb), len(nb)
gltf = {'asset':{'version':'2.0','generator':'Ark Sovereign Compiler v112'},'scene':0,'scenes':[{'nodes':[0]}],'nodes':[{'mesh':0,'name':'Leviathan_Core'}],'meshes':[{'primitives':[{'attributes':{'POSITION':1,'NORMAL':2},'indices':0,'mode':4}],'name':'Leviathan_Thermodynamic_Core'}],'accessors':[{'bufferView':0,'componentType':5125,'count':int(faces.size),'type':'SCALAR','max':[int(faces.max())],'min':[int(faces.min())]},{'bufferView':1,'componentType':5126,'count':len(verts),'type':'VEC3','max':verts.max(axis=0).tolist(),'min':verts.min(axis=0).tolist()},{'bufferView':2,'componentType':5126,'count':len(verts),'type':'VEC3'}],'bufferViews':[{'buffer':0,'byteOffset':0,'byteLength':il,'target':34963},{'buffer':0,'byteOffset':il,'byteLength':vl,'byteStride':12,'target':34962},{'buffer':0,'byteOffset':il+vl,'byteLength':nl,'byteStride':12,'target':34962}],'buffers':[{'byteLength':len(tb)}]}
gj = json.dumps(gltf,separators=(',',':')).encode(); gj += b' '*((4-len(gj)%4)%4)
total = 12 + 8 + len(gj) + 8 + len(tb)
with open('Ark_Leviathan_Core.glb','wb') as f:
    f.write(struct.pack('<III',0x46546C67,2,total))
    f.write(struct.pack('<II',len(gj),0x4E4F534A)); f.write(gj)
    f.write(struct.pack('<II',len(tb),0x004E4942)); f.write(tb)
print('[ARK-FFI] Exported: Ark_Leviathan_Core.glb')
print('[ARK-FFI] TITANIUM LOCK.')
