#!/bin/bash
# -*- mode: shell-script; coding: utf-8-unix -*-
#
# SPDX-License-Identifier: MIT
#
# SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026. All rights reserved.

#
# Extract 3D annotations from a PDF and dump them as raw U3D or PRC.
#
# for i in *.[pP][dD][fF]; do ~/dev/pdf3d/scripts/extract3d.sh "$i"; done
#

#set -x # echo
set -e # bail on first error

if [ $# -ne 1 ]; then
    echo "Error: Exactly 1 argument is required (input PDF file)."
    exit 1
fi

if ! command -v qpdf >/dev/null 2>&1
then
    echo "qpdf could not be found"
    exit 1
fi

if ! command -v jq >/dev/null 2>&1
then
    echo "jq could not be found"
    exit 1
fi

PDF="$1"
BASE="${PDF%.*}"
J="${BASE}.json"
echo -e "\n++ Processing $PDF..."

# turn pdf structure into JSON and dump embedded streams
qpdf "${PDF}" --json --json-stream-data=file --json-stream-prefix="${BASE}.stream" "$J" || true
#qpdf ${BASE}.pdf --json --json-stream-data=file --json-stream-prefix=${BASE}.stream | jq -r '.qpdf.[1] | to_entries [6:9] '
#qpdf ${BASE}.pdf --json --json-stream-data=file --json-stream-prefix=${BASE}.stream | jq -r '.qpdf.[1] | (. | has("value"))'
#qpdf ${BASE}.pdf --json --json-stream-data=file --json-stream-prefix=${BASE}.stream | jq -r '.qpdf.[1].[] | select((.value.["/Type"]? == "/Annot" ) and (.value.["/Subtype"]? == "/3D" ) ) '

# recursively parse attached PDFs for 3D content
NAT=`cat "$J" | jq -r '.attachments | length'`
echo "= Found $NAT Attachments in $PDF."
if [ $NAT -eq 0 ]; then
    #echo "Found 0 Attachments in $PDF."
    #echo ""
    :
else
    #echo "Found $NAT Attachments in $PDF."
    #qpdf "${BASE}.pdf" --list-attachments
    #qpdf "${BASE}.pdf" --list-attachments --verbose
    #cat "$J" | jq -r '.attachments '
    for ((I = 0 ; I < $NAT ; I++ )); do
        echo "= Extracting attachment id:$I to `pwd`/attachments"
        #qpdf "${BASE}.pdf" --list-attachments > ${BASE}.attachments.txt
        #qpdf "${BASE}.pdf" --list-attachments | sed "${i}q;d"
        #cat "$J" | jq -r ".attachments | to_entries.[$I] "
        #cat "$J" | jq -r ".attachments | to_entries.[$I] .key"
        #cat "$J" | jq -r ".attachments | to_entries.[$I] .value.preferredname"
        AT_KEY=`cat "$J" | jq -r ".attachments | to_entries.[$I] .key"`
        #echo "AT_KEY=$AT_KEY"
        AT_PREFNAME=`cat "$J" | jq -r ".attachments | to_entries.[$I] .value.preferredname"`
        #echo "AT_PREFNAME=$AT_PREFNAME"
        if [[ "$AT_PREFNAME" == *.pdf ]]; then
            AT_FN="${BASE}.attachment-${AT_PREFNAME}"
            #echo "AT_FN=$AT_FN"
            #J=$(expr $I + 1)
            #sed "${J}q;d" ${BASE}.attachments.txt 
            #sed "${J}q;d" ${BASE}.attachments.txt | cut -d " -> " -f 1
            #AT_KEY=`sed "${J}q;d" ${BASE}.attachments.txt`
            #AT_KEY=`qpdf "${BASE}.pdf" --list-attachments | sed "${i}q;d"`
            mkdir -p attachments
            qpdf "${PDF}" --show-attachment="${AT_KEY}" > attachments/"${AT_FN}"
        fi
    done
fi


# 3D Annotations
N3D=`cat "$J" | jq -r '.qpdf.[1] | to_entries .[] | select((.value.value.["/Type"]? == "/Annot" ) and (.value.value.["/Subtype"]? == "/3D" ) ) .key' | wc -l | xargs echo`
echo "= Found $N3D 3D Annotations in $PDF."

for ((i = 0 ; i < $N3D ; i++ )); do
    echo "= Extracting id:$i..."
    JI="${BASE}3D.$i.json"
    cat "$J" | jq -r ".qpdf.[1] | to_entries .[] | select((.value.value.[\"/Type\"]? == \"/Annot\" ) and (.value.value.[\"/Subtype\"]? == \"/3D\" ) )" | jq -s ".[$i]" > "$JI"
    #cat "$JI" | jq -r

    # 3DD : A 3D stream specifying the 3D content
    KEY3DD=`cat "$JI" | jq -r '.value.value.["/3DD"]'`
    #cat "$J" | jq -r ".qpdf.[1] | to_entries .[] | select(.key == \"obj:\"+\"$KEY3DD\" )"
    # filename of 3D stream
    F3DD=`cat "$J" | jq -r ".qpdf.[1] | to_entries .[] | select(.key == \"obj:\"+\"$KEY3DD\" ) .value.stream.datafile"`
    # file type of 3D data (filename extension)
    T3DD=`cat "$J" | jq -r ".qpdf.[1] | to_entries .[] | select(.key == \"obj:\"+\"$KEY3DD\" ) .value.stream.dict.[\"/Subtype\"]" | tr -d '/' | tr '[:upper:]' '[:lower:]'`
    echo "=  dumping $F3DD.$T3DD"
    mv "$F3DD" "$F3DD.$T3DD"
    
    # 3DV : The initial view of the 3D content
    KEY3DV=`cat "$JI" | jq -r '.value.value.["/3DV"]'`
    #cat "$J" | jq -r ".qpdf.[1] | to_entries .[] | select(.key == \"obj:\"+\"$KEY3DV\" )"

    # 3DA : The activation dictionary (optional)
done

# try to find 3D content based on magic bytes in dumped stream data
NIND=0
while IFS='' read -r -d '' filename; do
    : # something with "$filename"
#for i in "./${BASE}.stream-*"; do
    i=${filename}
    #echo "$i"
    if [[ "$i" == *.prc ]]; then
        continue
    fi
    if [[ "$i" == *.u3d ]]; then
        continue
    fi
    actualsize=$(wc -c <"$i")
    if [ $actualsize -ge 10 ]; then
        :
    else
        continue
    fi
    FIRST3=`hexdump -n 3 -C "${i}" | head -1 | cut -d '|' -f 2`
    case $FIRST3 in
        PRC)
            #echo "Found indirect PRC in second round"
            echo "=  dumping ${i}.prc"
            mv "$i" "$i".prc
            NIND=$(expr $NIND + 1)
            ;;
        U3D)
            #echo "Found indirect U3D in second round"
            echo "=  dumping ${i}.u3d"
            mv "$i" "$i".u3d
            NIND=$(expr $NIND + 1)
            ;;
    esac
#done
done < <(find . -maxdepth 1 -name "${BASE}.stream-*" -print0)
echo "= Found $NIND indirect 3D Annotations in ${PDF}"
    
# delete non-3D streams
find . \( -name "*\.stream-[[:digit:]][[:digit:]][[:digit:]][[:digit:]][[:digit:]]" -o -name "*\.stream-[[:digit:]][[:digit:]][[:digit:]][[:digit:]]" -o -name "*\.stream-[[:digit:]][[:digit:]][[:digit:]]" -o -name "*\.stream-[[:digit:]][[:digit:]]" -o -name "*\.stream-[[:digit:]]" \) -exec rm "{}" \;

#qpdf ${BASE}.pdf --json --json-stream-data=file --json-stream-prefix=${BASE}.stream | jq -r '.qpdf.[1]["obj:8 0 R"][]["\/Type"]'
#qpdf ${BASE}.pdf --json --json-stream-data=file --json-stream-prefix=${BASE}.stream | jq -r '.qpdf.[1]["obj:8 0 R"][]["\/Subtype"]'



