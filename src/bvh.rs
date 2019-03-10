
use hitable::*;
use std::rc::Rc;
use math::*;

pub struct BvhNode {
    left: Rc<Hitable>,
    right: Rc<Hitable>,
    bounding_box: AABB    
}

impl BvhNode {
    pub fn from_list(list: Vec<Rc<Hitable>>, time0: f64, time1: f64) -> BvhNode {

        let axis = (random::rand() * 3.0).floor() as u32; // SS: Choose random axis for simplicity

        let mut local_list = list;

        match axis {
            0 => local_list.sort_unstable_by(|a, b| {  
                if a.bounding_box(0.0, 0.0).min().x() - b.bounding_box(0.0, 0.0).min().x() < 0.0 {
                    return std::cmp::Ordering::Less;
                } else {
                    return std::cmp::Ordering::Greater;
                }
            }),
            1 => local_list.sort_unstable_by(|a, b| {  
                if a.bounding_box(0.0, 0.0).min().y() - b.bounding_box(0.0, 0.0).min().y() < 0.0 {
                    return std::cmp::Ordering::Less;
                } else {
                    return std::cmp::Ordering::Greater;
                }
            }),
            _ => local_list.sort_unstable_by(|a, b| {  
                if a.bounding_box(0.0, 0.0).min().z() - b.bounding_box(0.0, 0.0).min().z() < 0.0 {
                    return std::cmp::Ordering::Less;
                } else {
                    return std::cmp::Ordering::Greater;
                }
            }),
        };

        let left;
        let right;

        let list_length = local_list.len();
        if list_length == 1 {
            left = Rc::clone(&local_list[0]);
            right =  Rc::clone(&left);
        } else if list_length == 2 {
            left =  Rc::clone(&local_list[0]);
            right =  Rc::clone(&local_list[1]);
        } else {
            let half = list_length / 2;
            let second_half = local_list.split_off(half);
            left = Rc::new(BvhNode::from_list(local_list, time0, time1));
            right = Rc::new(BvhNode::from_list(second_half, time0, time1));
        }

        let box_left = left.bounding_box(time0, time1);
        let box_right = right.bounding_box(time0, time1);
        let bounding_box = AABB::get_union(&box_left, &box_right);
        
        BvhNode {
            left,
            right,
            bounding_box    
        }
    }
}

impl Hitable for BvhNode {
    
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        
        let mut record = None;
        if self.bounding_box.hit(ray, t_min, t_max) {

            let left_hit = self.left.hit(ray, t_min, t_max);
            let right_hit = self.right.hit(ray, t_min, t_max);

            // if both hits succeed return smallest time t, closet one. Else return the other one hit or none if no hit
            record = match left_hit {
                Some(left_hit_u) => match right_hit {
                    Some(right_hit_u) => if left_hit_u.t < right_hit_u.t { 
                        Some(left_hit_u)
                    } else {
                        Some(right_hit_u)
                    },
                    None => Some(left_hit_u)
                },
                None => match right_hit {
                    Some(right_hit_u) => Some(right_hit_u),
                    None => None
                },
            }
        } 
        
        return record;
    }

    fn bounding_box(&self, _t0: f64, _t1: f64) -> AABB {
        self.bounding_box.clone()
    }
}