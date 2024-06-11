""" Visualise the npz files in our dataset. """

import numpy as np
import os
import ffmpegio
from typing import Tuple
from palettable.scientific.sequential import Batlow_20

def binary_to_colour(frames: np.array, colour: Tuple[float, float, float]):
    """ Binary images must be either 0 or 1 in uint8. Colour is RGB in the 
    range 0 to 1.0."""
    frames = np.clip(frames, 0, 1)
    # Using memmap for these large videos that need conversion.
    coloured = np.zeros(shape=(*frames.shape, 3), dtype=np.uint8)

    # Take entries from RGB LUT according to greyscale values in image
    lut = [[0,0,0], [int(colour[0] * 255), int(colour[1] * 255), int(colour[2] * 255)]]
    np.take(lut, frames, axis=0, out=coloured)

    return coloured

def intensity_to_colour(frames: np.array, colourmap=Batlow_20):
    """Frames must be uint8 0 to 255."""
    assert(np.max(frames) - np.min(frames) > 1)
    assert(frames.dtype == np.uint8)
    # Using memmap for these large videos that need conversion.
    coloured = np.zeros(shape=(*frames.shape, 3), dtype=np.uint8)

    # Take entries from RGB LUT according to greyscale values in image
    lut = [colourmap.mpl_colormap(x / 255.0) for x in range(256)]
    lut = [[int(x[0] * 255), int(x[1] * 255), int(x[2] * 255)] for x in lut]
    np.take(lut, frames, axis=0, out=coloured)

    return coloured

def add_blend(fg, bg, alpha=1.0):
    """Additive blend but sticking in the 255 space."""
    assert(alpha > 0 and alpha <= 1.0)
    assert(fg.dtype == np.uint8)
    assert(bg.dtype == np.uint8)
    final = bg
    mixed = (fg * alpha).astype(np.uint8)
    final += mixed
    final[final < mixed]=255

    return final

def main(args):
    """ Load the base npz and, optionally, the mask NPZ as well. Colourise them
    and combine if necessary, then create an mp4 video of the result."""
    assert(os.path.exists(args.base))
    x = np.load(args.base)
    x = intensity_to_colour(x)
    y = None

    if args.mask is not None:
        assert(os.path.exists(args.mask))
        y = np.load(args.mask)
        y = np.where( y >= 1, 1, 0).astype(np.uint8)
        y = binary_to_colour(y, [1.0, 0.0, 0.0])

    if y is not None:
        x = add_blend(x, y, 0.8)
        x = np.clip(x, 0, 255).astype(np.uint8)

    outpath = os.path.basename(args.base) + ".mp4"
    ffmpegio.video.write(os.path.join(args.outpath, outpath), 4, x, overwrite=True, show_log=False)

if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        prog="crabseal - vis.py",
        description="Visualise the npz files to check.",
        epilog="SMRU St Andrews",
    )
    parser.add_argument(
        "-b", "--base", default=".", help="The path to the saved base file."
    )
    parser.add_argument(
        "-m", "--mask", default=None, help="[optional] The path to the saved mask file."
    )
    parser.add_argument("-o", "--outpath", default=".", help="The path for the output.")
   
    args = parser.parse_args()
    main(args)